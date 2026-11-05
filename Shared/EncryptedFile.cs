using System.Security.Cryptography;
using System.Text.Json;

namespace LicenseVault;

public static class EncryptedFile
{
    // Version 1: magic (4), salt (16), nonce (12), tag (16), ciphertext.
    public static byte[] Encrypt(byte[] plaintext, string password)
    {
        var salt = RandomNumberGenerator.GetBytes(16);
        var key = Rfc2898DeriveBytes.Pbkdf2(password, salt, 600000, HashAlgorithmName.SHA256, 32);
        try
        {
            var data = new byte[48 + plaintext.Length];
            "LKV1"u8.CopyTo(data); salt.CopyTo(data, 4);
            RandomNumberGenerator.Fill(data.AsSpan(20, 12));
            using var aes = new AesGcm(key, 16);
            aes.Encrypt(data.AsSpan(20, 12), plaintext, data.AsSpan(48), data.AsSpan(32, 16), data.AsSpan(0, 20));
            return data;
        }
        finally { CryptographicOperations.ZeroMemory(key); }
    }
    public static byte[] Decrypt(byte[] data, string password)
    {
        if (data.Length < 48 || !data.AsSpan(0, 4).SequenceEqual("LKV1"u8)) throw new InvalidDataException("Geen ondersteunde kluis.");
        var key = Rfc2898DeriveBytes.Pbkdf2(password, data.AsSpan(4, 16), 600000, HashAlgorithmName.SHA256, 32);
        try
        {
            var plain = new byte[data.Length - 48];
            using var aes = new AesGcm(key, 16);
            aes.Decrypt(data.AsSpan(20, 12), data.AsSpan(48), data.AsSpan(32, 16), plain, data.AsSpan(0, 20));
            return plain;
        }
        finally { CryptographicOperations.ZeroMemory(key); }
    }
    public static T Load<T>(string path, string password) where T : new()
    {
        if (!File.Exists(path)) return new T();
        var plain = Decrypt(File.ReadAllBytes(path), password);
        try { return JsonSerializer.Deserialize<T>(plain) ?? throw new InvalidDataException("Lege kluis."); }
        finally { CryptographicOperations.ZeroMemory(plain); }
    }
    public static void Save<T>(string path, T value, string password)
    {
        var plain = JsonSerializer.SerializeToUtf8Bytes(value);
        byte[] encrypted;
        try { encrypted = Encrypt(plain, password); }
        finally { CryptographicOperations.ZeroMemory(plain); }
        var temp = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
        try
        {
            using (var stream = new FileStream(temp, FileMode.CreateNew, FileAccess.Write, FileShare.None, 4096, FileOptions.WriteThrough))
            { stream.Write(encrypted); stream.Flush(true); }
            if (File.Exists(path)) File.Copy(path, path + ".bak", true);
            File.Move(temp, path, true);
        }
        finally { if (File.Exists(temp)) File.Delete(temp); }
    }
}
