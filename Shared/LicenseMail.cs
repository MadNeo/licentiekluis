using System.Net.Mail;
using System.Text;

namespace LicenseVault;
public record MailSelection(Guid Id, long Revision);
public record LicenseMailRequest(Guid RequestId, string Recipient, string Subject, List<MailSelection> Licenses);
public record MailOutcome(string Status, string Message);
public record MailAttempt(string Fingerprint, string Status, DateTimeOffset Created);
public record SmtpSettings(string Host, int Port, string From, string Username, string Password);

public static class LicenseMail
{
    public static string Address(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Length > 254 || value.Contains('\r') || value.Contains('\n'))
            throw new ArgumentException("Vul een geldig e-mailadres in.");
        var trimmed = value.Trim();
        if (!MailAddress.TryCreate(trimmed, out var address) || address.Address != trimmed || !address.Address.Contains('@'))
            throw new ArgumentException("Gebruik precies een e-mailadres, zonder naam, CC of BCC.");
        return address.Address;
    }
    public static List<Entry> Select(LicenseMailRequest request, IEnumerable<Entry> all)
    {
        _ = Address(request.Recipient);
        if (request.RequestId == Guid.Empty || string.IsNullOrWhiteSpace(request.Subject) || request.Subject.Length > 150 || request.Subject.Contains('\r') || request.Subject.Contains('\n'))
            throw new ArgumentException("Ongeldig onderwerp of verzoeknummer.");
        if (request.Licenses is null || request.Licenses.Count is < 1 or > 50 || request.Licenses.Select(x => x.Id).Distinct().Count() != request.Licenses.Count)
            throw new ArgumentException("Selecteer 1 tot 50 verschillende licenties.");
        var map = all.ToDictionary(e => e.Id);
        var selected = new List<Entry>();
        foreach (var item in request.Licenses)
        {
            if (!map.TryGetValue(item.Id, out var entry) || entry.Deleted || entry.Kind != "license")
                throw new ArgumentException("Een geselecteerde licentie bestaat niet meer. Vernieuw de selectie.");
            if (entry.Revision != item.Revision)
                throw new ArgumentException("Een licentie is gewijzigd. Vernieuw en controleer de selectie opnieuw.");
            if (string.IsNullOrWhiteSpace(entry.Code)) throw new ArgumentException($"Licentie '{entry.Name}' heeft geen code.");
            selected.Add(entry);
        }
        return selected;
    }
    public static string Body(IEnumerable<Entry> selected)
    {
        var body = new StringBuilder("Beste ontvanger,\r\n\r\nHierbij de geselecteerde softwarelicenties.\r\n\r\n");
        var number = 1;
        foreach (var e in selected)
        {
            body.AppendLine($"{number++}. Software: {e.Name}");
            if (!string.IsNullOrWhiteSpace(e.SoftwareVersion)) body.AppendLine($"Versie: {e.SoftwareVersion}");
            body.AppendLine($"Licentiecode: {e.Code}");
            body.AppendLine();
        }
        body.AppendLine("Bewaar deze gegevens zorgvuldig.");
        body.AppendLine("Verstuurd vanuit LicentieKluis.");
        // No notes, device names, user names, vault password or SMTP credentials.
        return body.ToString().Replace("\r\n", "\n").Replace("\n", "\r\n");
    }
}
