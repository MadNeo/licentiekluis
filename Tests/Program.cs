using LicenseVault;
using System.Security.Cryptography;

int checks = 0;
void Check(bool ok, string name) { if (!ok) throw new Exception("FAIL: " + name); checks++; Console.WriteLine("PASS: " + name); }
var server = new ServerState();
var local = new LocalState { VaultId = server.VaultId };
var person = new Entry { Kind = "person", Name = "Testgebruiker" };
var license = new Entry { Name = "Testsoftware", OwnerId = person.Id, Code = "TEST-NOT-REAL", Installations = [new("Windows", "PC1"), new("Linux", "VM1")] };
SyncEngine.Edit(local, person); SyncEngine.Edit(local, license);
SyncRequest Request(LocalState l) => new(server.VaultId, l.Pending.Select(p => new Change(l.Entries.Single(e => e.Id == p.Key), p.Value)).ToList());
var request = Request(local);
var initial = SyncEngine.Apply(server, request); server = initial.State;
Check(server.Entries.Count == 2 && server.Entries.All(e => e.Revision == 1), "offline create and first upload");
var replay = SyncEngine.Apply(server, request); server = replay.State;
Check(server.Entries.All(e => e.Revision == 1) && replay.Response.Conflicts.Count == 0, "retry after lost response is idempotent");
SyncEngine.Merge(local, replay.Response);
Check(local.Pending.Count == 0, "successful merge clears pending");
var second = new LocalState { VaultId = server.VaultId };
SyncEngine.Merge(second, new SyncResponse(server.Entries, [], []));
SyncEngine.Edit(local, local.Entries.Single(e => e.Id == license.Id) with { Name = "Edit A" });
SyncEngine.Edit(second, second.Entries.Single(e => e.Id == license.Id) with { Name = "Edit B" });
var a = SyncEngine.Apply(server, Request(local)); server = a.State; SyncEngine.Merge(local, a.Response);
var b = SyncEngine.Apply(server, Request(second)); server = b.State; SyncEngine.Merge(second, b.Response);
Check(second.Conflicts.Count == 1 && second.Entries.Single(e => e.Id == license.Id).Name == "Edit B", "concurrent edit retains local and remote");
SyncEngine.Resolve(second, license.Id, true);
var resolved = SyncEngine.Apply(server, Request(second)); server = resolved.State; SyncEngine.Merge(second, resolved.Response);
Check(server.Entries.Single(e => e.Id == license.Id).Name == "Edit B" && second.Pending.Count == 0, "explicit keep-local conflict resolution");
SyncEngine.Edit(local, local.Entries.Single(e => e.Id == license.Id) with { Deleted = true });
var deletion = SyncEngine.Apply(server, Request(local)); SyncEngine.Merge(local, deletion.Response);
Check(local.Conflicts.Count == 1, "stale deletion cannot erase newer edit");
SyncEngine.Resolve(local, license.Id, false);
Check(!local.Entries.Single(e => e.Id == license.Id).Deleted && local.Pending.Count == 0, "keep-server resolution");
SyncEngine.Edit(local, local.Entries.Single(e => e.Id == license.Id) with { Deleted = true });
var deleted = SyncEngine.Apply(server, Request(local)); server = deleted.State;
Check(server.Entries.Single(e => e.Id == license.Id).Deleted, "deletion preserved as tombstone");
var before = server.Entries.Count;
try { SyncEngine.Apply(server, new SyncRequest(Guid.NewGuid(), [])); Check(false, "wrong vault accepted"); } catch (ArgumentException) { Check(true, "wrong vault rejected"); }
try { SyncEngine.Apply(server, new SyncRequest(server.VaultId, [new(new Entry { Name = "Invalid", OwnerId = Guid.NewGuid() }, 0)])); Check(false, "orphan accepted"); } catch (ArgumentException) { Check(server.Entries.Count == before, "orphan rejected without partial state mutation"); }
try { SyncEngine.Validate(license with { Installations = [new("Invalid", "PC")] }); Check(false, "platform accepted"); } catch (ArgumentException) { Check(true, "invalid platform rejected"); }
const string password = "test-only-long-passphrase";
var plain = "confidential license"u8.ToArray();
var cipher = EncryptedFile.Encrypt(plain, password);
Check(EncryptedFile.Decrypt(cipher, password).SequenceEqual(plain), "encryption roundtrip");
Check(!cipher.SequenceEqual(EncryptedFile.Encrypt(plain, password)), "random salt and nonce");
try { EncryptedFile.Decrypt(cipher, "wrong"); Check(false, "wrong password accepted"); } catch (CryptographicException) { Check(true, "wrong password rejected"); }
cipher[^1] ^= 1;
try { EncryptedFile.Decrypt(cipher, password); Check(false, "tamper accepted"); } catch (CryptographicException) { Check(true, "tamper detected"); }
var testDir = Path.Combine(Environment.CurrentDirectory, "lkv-test-" + Guid.NewGuid().ToString("N"));
Directory.CreateDirectory(testDir);
try
{
    var file = Path.Combine(testDir, "test.lkv");
    EncryptedFile.Save(file, second, password); EncryptedFile.Save(file, local, password);
    Check(File.Exists(file + ".bak"), "previous encrypted version retained");
    Check(EncryptedFile.Load<LocalState>(file, password).Entries.Count == local.Entries.Count, "durable file roundtrip");
}
finally { Directory.Delete(testDir, true); }
var ma = new Entry { Kind = "license", Name = "Software A", Code = "AAA-BBB", SoftwareVersion = "2026", Notes = "PRIVATE", Revision = 1 };
var mb = ma with { Id = Guid.NewGuid(), Name = "Software B", Code = "CCC-DDD" };
var mr = new LicenseMailRequest(Guid.NewGuid(), "test@example.org", "Licenties", [new(ma.Id, 1), new(mb.Id, 1)]);
var body = LicenseMail.Body(LicenseMail.Select(mr, [ma, mb]));
Check(body.Contains("Software A") && body.Contains("AAA-BBB") && body.Contains("Software B") && body.Contains("CCC-DDD") && !body.Contains("PRIVATE"), "mail contains selected software/code pairs without private notes");
void RejectMail(LicenseMailRequest req, IEnumerable<Entry> values, string name) { try { LicenseMail.Select(req, values); Check(false, name); } catch (ArgumentException) { Check(true, name); } }
RejectMail(mr with { Recipient = "test@example.org\r\nBcc: bad@example.org" }, [ma,mb], "mail rejects recipient header injection");
RejectMail(mr with { Subject = "Hello\r\nBcc: bad@example.org" }, [ma,mb], "mail rejects subject header injection");
RejectMail(mr with { Licenses = [new(ma.Id,1),new(ma.Id,1)] }, [ma,mb], "mail rejects duplicate selection");
RejectMail(mr, [ma with { Revision = 2 },mb], "mail rejects stale preview");
RejectMail(mr, [ma with { Deleted = true },mb], "mail rejects deleted license");
RejectMail(mr, [ma with { Code = "" },mb], "mail rejects missing code");
RejectMail(mr with { Licenses = [] }, [ma,mb], "mail rejects empty selection");
Console.WriteLine($"{checks} checks passed.");
if (args.Contains("--fixture"))
{
    var fixturePath = Path.GetFullPath("LicenseVault-Rust/fixtures/dotnet-vault.lkv");
    Directory.CreateDirectory(Path.GetDirectoryName(fixturePath)!);
    EncryptedFile.Save(fixturePath, local, "test-only-long-passphrase");
    Console.WriteLine("Synthetic .NET compatibility fixture created.");
}
