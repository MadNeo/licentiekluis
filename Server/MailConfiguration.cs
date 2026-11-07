using LicenseVault;
using System.Security.Cryptography;

// Accessed while holding mailGate. The password is never part of a response.
sealed class MailConfiguration(string path, string key)
{
    public SmtpSettings? Load() => File.Exists(path)
        ? EncryptedFile.Load<SmtpSettingsStore>(path, key).ToSettings() : null;

    public string Revision => File.Exists(path)
        ? Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))) : "new";

    public MailSettingsView View()
    {
        var value = Load();
        return new(value is not null, value?.Host ?? "", value?.Port ?? 587,
            value?.From ?? "", value?.Username ?? "", !string.IsNullOrEmpty(value?.Password), Revision);
    }

    public MailSettingsView Save(MailSettingsUpdate update)
    {
        RequireRevision(update.Revision);
        var previous = Load();
        var host = (update.Host ?? "").Trim();
        var username = (update.Username ?? "").Trim();
        var password = update.Password ?? "";
        if (update.ClearPassword && password.Length > 0)
            throw new ArgumentException("Kies een nieuw wachtwoord of verwijder het opgeslagen wachtwoord.");
        if (!update.ClearPassword && password.Length == 0 && previous is not null)
        {
            if (previous.Password.Length > 0 && (host != previous.Host || update.Port != previous.Port || username != previous.Username))
                throw new ArgumentException("Voer het SMTP-wachtwoord opnieuw in wanneer je de mailserver, poort of gebruikersnaam wijzigt.");
            password = previous.Password;
        }
        var settings = new SmtpSettings(host, update.Port, LicenseMail.Address(update.From ?? ""), username, password);
        Validate(settings);
        EncryptedFile.Save(path, settings, key);
        if (!OperatingSystem.IsWindows())
        {
            File.SetUnixFileMode(path, UnixFileMode.UserRead | UnixFileMode.UserWrite);
            if (File.Exists(path + ".bak")) File.SetUnixFileMode(path + ".bak", UnixFileMode.UserRead | UnixFileMode.UserWrite);
        }
        return View();
    }

    public void RequireRevision(string? revision)
    {
        if (revision != Revision) throw new SettingsChangedException();
    }

    public static void Validate(SmtpSettings settings)
    {
        if (settings.Host.Length is 0 or > 253 || Uri.CheckHostName(settings.Host) == UriHostNameType.Unknown)
            throw new ArgumentException("Vul een geldige SMTP-host in, zonder https:// of pad.");
        if (settings.Port is < 1 or > 65535 || settings.Port == 465)
            throw new ArgumentException("Gebruik een STARTTLS-poort, meestal 587. Impliciete TLS op poort 465 wordt niet ondersteund.");
        _ = LicenseMail.Address(settings.From);
        if (settings.Username.Length > 254 || settings.Username.Any(char.IsControl) || settings.Password.Length > 1024 || settings.Password.Any(char.IsControl))
            throw new ArgumentException("De SMTP-gebruikersnaam of het wachtwoord is ongeldig of te lang.");
        if ((settings.Username.Length > 0) != (settings.Password.Length > 0))
            throw new ArgumentException("Vul zowel de SMTP-gebruikersnaam als het wachtwoord in, of kies een relay zonder aanmelding.");
    }
}

sealed class SettingsChangedException : Exception {}
record MailSettingsView(bool Configured, string Host, int Port, string From, string Username, bool PasswordSaved, string Revision);
record MailSettingsUpdate(string? Host, int Port, string? From, string? Username, string? Password, bool ClearPassword, string? Revision);
record MailTestRequest(Guid RequestId, string? Recipient, string? Revision);
