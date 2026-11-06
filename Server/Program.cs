using LicenseVault;
using System.Collections.Concurrent;
using System.Security.Cryptography;
using System.Text.Json;
using System.Threading.RateLimiting;
using Microsoft.AspNetCore.RateLimiting;
using System.Net;
using System.Net.Mail;
using System.Text;

var dataDir = Environment.GetEnvironmentVariable("LKV_DATA") ?? Path.Combine(AppContext.BaseDirectory, "data");
Directory.CreateDirectory(dataDir);
var configPath = Path.Combine(dataDir, "secrets.json");
if (args.Contains("--init") || args.Contains("--init-stdin"))
{
    if (File.Exists(configPath)) throw new InvalidOperationException("Configuratie bestaat al; niet overschreven.");
    Console.Write("Beheerdersnaam: "); var user = Console.ReadLine()?.Trim() ?? "";
    Console.Write("Wachtwoord (minimaal 14 tekens): "); var password = args.Contains("--init-stdin") ? Console.ReadLine() ?? "" : ReadPassword();
    Console.Write("Herhaal wachtwoord: "); var repeat = args.Contains("--init-stdin") ? Console.ReadLine() ?? "" : ReadPassword();
    if (user.Length == 0 || password.Length < 14 || password != repeat) throw new ArgumentException("Naam of wachtwoord ongeldig.");
    var salt = RandomNumberGenerator.GetBytes(16);
    var auth = new Config(user, Convert.ToBase64String(salt), Convert.ToBase64String(Rfc2898DeriveBytes.Pbkdf2(password, salt, 600000, HashAlgorithmName.SHA256, 32)), Convert.ToBase64String(RandomNumberGenerator.GetBytes(32)));
    File.WriteAllText(configPath, JsonSerializer.Serialize(auth));
    if (!OperatingSystem.IsWindows()) File.SetUnixFileMode(configPath, UnixFileMode.UserRead | UnixFileMode.UserWrite);
    Console.WriteLine("Serveraccount aangemaakt."); return;
}
var config = JsonSerializer.Deserialize<Config>(File.ReadAllText(configPath)) ?? throw new InvalidDataException("Configuratie ontbreekt.");
var smtpPath = Path.Combine(dataDir, "smtp.lkv");
if (args.Contains("--configure-mail"))
{
    Console.WriteLine("SMTP met STARTTLS (gewoonlijk poort 587). Poort 465 en OAuth zijn niet ondersteund.");
    Console.Write("SMTP-host: "); var host = Console.ReadLine()?.Trim() ?? "";
    Console.Write("SMTP-poort [587]: "); var portText = Console.ReadLine(); var port = string.IsNullOrWhiteSpace(portText) ? 587 : int.Parse(portText);
    Console.Write("Afzenderadres: "); var from = LicenseMail.Address(Console.ReadLine() ?? "");
    Console.Write("SMTP-gebruikersnaam (leeg voor vertrouwde relay): "); var username = Console.ReadLine()?.Trim() ?? "";
    Console.Write("SMTP-wachtwoord / app-wachtwoord: "); var pass = ReadPassword();
    if (Uri.CheckHostName(host) == UriHostNameType.Unknown || port is < 1 or > 65535 || port == 465) throw new ArgumentException("Ongeldige host of poort. Gebruik STARTTLS.");
    EncryptedFile.Save(smtpPath, new SmtpSettings(host, port, from, username, pass), config.Key);
    if (!OperatingSystem.IsWindows()) File.SetUnixFileMode(smtpPath, UnixFileMode.UserRead | UnixFileMode.UserWrite);
    Console.WriteLine("Mailinstellingen versleuteld opgeslagen. Er is geen testmail verstuurd."); return;
}
var vaultPath = Path.Combine(dataDir, "server.lkv");
var state = EncryptedFile.Load<ServerState>(vaultPath, config.Key);
EncryptedFile.Save(vaultPath, state, config.Key);
var gate = new SemaphoreSlim(1, 1);
var sessions = new ConcurrentDictionary<string, DateTimeOffset>();
var mailGate = new SemaphoreSlim(1, 1);
var mailLogPath = Path.Combine(dataDir, "mail-log.lkv");
var mailLog = EncryptedFile.Load<Dictionary<Guid, MailAttempt>>(mailLogPath, config.Key);
var builder = WebApplication.CreateBuilder(args);
builder.WebHost.ConfigureKestrel(o => o.Limits.MaxRequestBodySize = 16 * 1024 * 1024);
builder.Services.AddRateLimiter(o =>
{
    o.RejectionStatusCode = 429;
    // Global limit also works behind a local TLS proxy without trusting spoofable headers.
    o.AddFixedWindowLimiter("login", p => { p.PermitLimit = 10; p.Window = TimeSpan.FromMinutes(1); p.QueueLimit = 0; });
    o.AddFixedWindowLimiter("mail", p => { p.PermitLimit = 5; p.Window = TimeSpan.FromMinutes(1); p.QueueLimit = 0; });
});
var app = builder.Build();
app.Use(async (ctx, next) =>
{
    ctx.Response.Headers.CacheControl = "no-store";
    ctx.Response.Headers.XContentTypeOptions = "nosniff";
    ctx.Response.Headers["Content-Security-Policy"] = "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'";
    ctx.Response.Headers["Referrer-Policy"] = "no-referrer";
    if (ctx.Request.Path.StartsWithSegments("/api") && ctx.Request.Path != "/api/login")
    {
        var bearer = ctx.Request.Headers.Authorization.ToString();
        if (!bearer.StartsWith("Bearer ", StringComparison.Ordinal) || !sessions.TryGetValue(bearer[7..], out var expires) || expires <= DateTimeOffset.UtcNow)
        { ctx.Response.StatusCode = 401; return; }
    }
    await next();
});
app.UseRateLimiter();
var webRoot = Path.Combine(AppContext.BaseDirectory, "wwwroot");
app.MapGet("/", () => Results.File(Path.Combine(webRoot, "index.html"), "text/html; charset=utf-8"));
app.MapGet("/theme.js", () => Results.File(Path.Combine(webRoot, "theme.js"), "text/javascript; charset=utf-8"));
app.MapGet("/changelog.js", () => Results.File(Path.Combine(webRoot, "changelog.js"), "text/javascript; charset=utf-8"));
app.MapGet("/changelog", () => Results.File(Path.Combine(webRoot, "changelog.html"), "text/html; charset=utf-8"));
app.MapGet("/changelog.txt", () => Results.File(Path.Combine(webRoot, "changelog.txt"), "text/plain; charset=utf-8"));
app.MapGet("/vault.js", () => Results.File(Path.Combine(webRoot, "vault.js"), "text/javascript"));
app.MapGet("/manage.js", () => Results.File(Path.Combine(webRoot, "manage.js"), "text/javascript"));
app.MapGet("/app.js", () => Results.File(Path.Combine(webRoot, "app.js"), "text/javascript; charset=utf-8"));
app.MapGet("/mail.js", () => Results.File(Path.Combine(webRoot, "mail.js"), "text/javascript; charset=utf-8"));
app.MapGet("/app.css", () => Results.File(Path.Combine(webRoot, "app.css"), "text/css; charset=utf-8"));
app.MapGet("/health", () => Results.Ok(new { status = "ok", version = "0.7.0" }));
app.MapGet("/api/mail/status", () => Results.Ok(new { configured = File.Exists(smtpPath) }));
app.MapPost("/api/mail/send", async (LicenseMailRequest request) =>
{
    await mailGate.WaitAsync();
    try
    {
        if (!File.Exists(smtpPath)) return Results.Json(new { error = "SMTP is nog niet ingesteld. Voer configure-mail.sh uit op de server." }, statusCode: 503);
        var fingerprint = Convert.ToHexString(SHA256.HashData(JsonSerializer.SerializeToUtf8Bytes(request)));
        if (mailLog.TryGetValue(request.RequestId, out var prior))
        {
            if (prior.Fingerprint != fingerprint) return Results.Conflict(new { error = "Dit verzoeknummer hoort bij een andere mail." });
            return Results.Ok(new MailOutcome(prior.Status == "submitted" ? "submitted" : "unknown", prior.Status == "submitted" ? "De mail is al aan de mailserver aangeboden; niet opnieuw verzonden." : "Eerdere verzendpoging heeft een onzekere status. Controleer eerst of de ontvanger de mail heeft ontvangen."));
        }
        List<Entry> selected;
        await gate.WaitAsync();
        try { selected = LicenseMail.Select(request, state.Entries); }
        finally { gate.Release(); }
        var settings = EncryptedFile.Load<SmtpSettingsStore>(smtpPath, config.Key).ToSettings();
        using var message = new MailMessage(LicenseMail.Address(settings.From), LicenseMail.Address(request.Recipient))
        { Subject = request.Subject.Trim(), Body = LicenseMail.Body(selected), BodyEncoding = Encoding.UTF8, SubjectEncoding = Encoding.UTF8, IsBodyHtml = false };
        using var smtp = new SmtpClient(settings.Host, settings.Port) { EnableSsl = true, UseDefaultCredentials = false, DeliveryMethod = SmtpDeliveryMethod.Network };
        if (settings.Username.Length > 0) smtp.Credentials = new NetworkCredential(settings.Username, settings.Password);
        mailLog[request.RequestId] = new MailAttempt(fingerprint, "sending", DateTimeOffset.UtcNow);
        EncryptedFile.Save(mailLogPath, mailLog, config.Key); // Durable before SMTP: no blind retry after a crash.
        try
        {
            using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(30));
            await smtp.SendMailAsync(message, timeout.Token);
            mailLog[request.RequestId] = mailLog[request.RequestId] with { Status = "submitted" };
            EncryptedFile.Save(mailLogPath, mailLog, config.Key);
            return Results.Ok(new MailOutcome("submitted", "Aangeboden aan de mailserver. Dit is geen bevestiging van ontvangst."));
        }
        catch (Exception e) when (e is SmtpException or OperationCanceledException or IOException)
        {
            mailLog[request.RequestId] = mailLog[request.RequestId] with { Status = "unknown" };
            EncryptedFile.Save(mailLogPath, mailLog, config.Key);
            return Results.Ok(new MailOutcome("unknown", "Verzending niet bevestigd. Controleer SMTP-instellingen en ontvangst voordat je een nieuwe verzendpoging start."));
        }
    }
    catch (ArgumentException e) { return Results.BadRequest(new { error = e.Message }); }
    finally { mailGate.Release(); }
}).RequireRateLimiting("mail");
app.MapGet("/api/entries", async () =>
{
    await gate.WaitAsync();
    try { return Results.Ok(state.Entries.Where(e => !e.Deleted).Select(e => e with { Code = "" }).ToList()); }
    finally { gate.Release(); }
});
app.MapGet("/api/licenses/{id:guid}/code", async (Guid id) =>
{
    await gate.WaitAsync();
    try
    {
        var e = state.Entries.FirstOrDefault(e => e.Id == id && e.Kind == "license" && !e.Deleted);
        return e is null ? Results.NotFound() : Results.Ok(new { code = e.Code });
    }
    finally { gate.Release(); }
});
app.MapPost("/api/login", (LoginRequest login) =>
{
    if (string.IsNullOrEmpty(login.Password) || login.Password.Length > 1024) return Results.Unauthorized();
    var hash = Rfc2898DeriveBytes.Pbkdf2(login.Password, Convert.FromBase64String(config.Salt), 600000, HashAlgorithmName.SHA256, 32);
    var valid = CryptographicOperations.FixedTimeEquals(hash, Convert.FromBase64String(config.Hash));
    CryptographicOperations.ZeroMemory(hash);
    if (!valid || login.Username != config.Username) return Results.Unauthorized();
    foreach (var pair in sessions.Where(p => p.Value <= DateTimeOffset.UtcNow)) sessions.TryRemove(pair.Key, out _);
    if (sessions.Count >= 1000) return Results.StatusCode(429);
    var token = Convert.ToHexString(RandomNumberGenerator.GetBytes(32));
    sessions[token] = DateTimeOffset.UtcNow.AddHours(8);
    return Results.Ok(new LoginResponse(token, state.VaultId));
}).RequireRateLimiting("login");
app.MapPost("/api/logout", (HttpContext ctx) => { sessions.TryRemove(ctx.Request.Headers.Authorization.ToString()[7..], out _); return Results.Ok(); });
app.MapPost("/api/sync", async (SyncRequest request) =>
{
    await gate.WaitAsync();
    try
    {
        var (next, response) = SyncEngine.Apply(state, request);
        if (request.Changes.Count > 0) EncryptedFile.Save(vaultPath, next, config.Key);
        state = next;
        return Results.Ok(response);
    }
    catch (ArgumentException e) { return Results.BadRequest(new { error = e.Message }); }
    finally { gate.Release(); }
});
app.Run();

static string ReadPassword()
{
    var s = new System.Text.StringBuilder();
    ConsoleKeyInfo k;
    while ((k = Console.ReadKey(true)).Key != ConsoleKey.Enter)
    { if (k.Key == ConsoleKey.Backspace) { if (s.Length > 0) s.Length--; } else if (!char.IsControl(k.KeyChar)) s.Append(k.KeyChar); }
    Console.WriteLine(); return s.ToString();
}
record Config(string Username, string Salt, string Hash, string Key);
// Parameterless type for the existing encrypted-file loader. Never sent to clients.
class SmtpSettingsStore
{
    public string Host { get; set; } = "";
    public int Port { get; set; } = 587;
    public string From { get; set; } = "";
    public string Username { get; set; } = "";
    public string Password { get; set; } = "";
    public SmtpSettings ToSettings() => new(Host, Port, From, Username, Password);
}

