using System.Text;
namespace LicenseVault;

public record RecoveryTextFile(string Name, string Text);
public record RecoveryPhoto(string Name, string MediaType, string Data);
public record RecoveryData
{
    public string Category { get; init; } = "authenticator";
    public string Text { get; init; } = "";
    public RecoveryTextFile? TextFile { get; init; }
    public RecoveryPhoto? Photo { get; init; }
    public const int MaxPhoto = 2 * 1024 * 1024;
    public const int MaxTextFile = 64 * 1024;
    public const int MaxTotal = 8 * 1024 * 1024;
    static bool FileName(string? name) => !string.IsNullOrWhiteSpace(name) && name.Length <= 200 && name is not ("." or "..") && !name.Any(c => char.IsControl(c) || "/\\:".Contains(c));
    public void Validate(bool deleted = false)
    {
        if (Category is not ("authenticator" or "wallet") || Text is null || Text.Length > 20000 || Text.Contains('\0'))
            throw new ArgumentException("Kies een categorie en gebruik maximaal 20.000 tekens hersteltekst.");
        if (TextFile is { } f && (!FileName(f.Name) || !f.Name.EndsWith(".txt", StringComparison.OrdinalIgnoreCase) || string.IsNullOrWhiteSpace(f.Text) || Encoding.UTF8.GetByteCount(f.Text) > MaxTextFile || f.Text.Contains('\0')))
            throw new ArgumentException("Kies een niet-leeg UTF-8-tekstbestand (.txt, maximaal 64 KB).");
        if (Photo is { } p)
        {
            if (!FileName(p.Name) || p.Data is null || p.Data.Length > ((MaxPhoto + 2) / 3) * 4)
                throw new ArgumentException("Foto ongeldig of groter dan 2 MB.");
            byte[] bytes;
            try { bytes = Convert.FromBase64String(p.Data); }
            catch (FormatException) { throw new ArgumentException("Ongeldige foto."); }
            try {
                var valid = p.MediaType switch {
                    "image/png" => bytes.AsSpan().StartsWith(new byte[] {137,80,78,71,13,10,26,10}),
                    "image/jpeg" => bytes.AsSpan().StartsWith(new byte[] {255,216,255}), _ => false };
                if (!valid || bytes.Length > MaxPhoto || Convert.ToBase64String(bytes) != p.Data)
                    throw new ArgumentException("Kies een PNG- of JPEG-foto (maximaal 2 MB).");
            } finally { System.Security.Cryptography.CryptographicOperations.ZeroMemory(bytes); }
        }
        if (!deleted && string.IsNullOrWhiteSpace(Text) && TextFile is null && Photo is null)
            throw new ArgumentException("Voeg tekst, een tekstbestand of een foto toe. Eén vorm is voldoende.");
    }
    [System.Text.Json.Serialization.JsonIgnore]
    public long Size => Encoding.UTF8.GetByteCount(Text) + (TextFile is null ? 0 : Encoding.UTF8.GetByteCount(TextFile.Text)) + (Photo?.Data.Length ?? 0);
    public static void ValidateTotal(IEnumerable<Entry> entries) {
        if (entries.Sum(e => e.Recovery?.Size ?? 0) > MaxTotal) throw new ArgumentException("Herstelgegevens samen maximaal 8 MB. Verwijder een ongebruikte foto of kies een kleinere foto.");
    }
}
