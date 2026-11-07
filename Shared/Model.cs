namespace LicenseVault;

public record Installation(string Platform, string Device);
public record Entry
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public RecoveryData? Recovery { get; init; }
    public string Kind { get; init; } = "license";
    public string Name { get; init; } = "";
    public Guid? OwnerId { get; init; }
    public string Code { get; init; } = "";
    public string SoftwareVersion { get; init; } = "";
    public string Notes { get; init; } = "";
    public DateOnly? Expires { get; init; }
    public List<Installation> Installations { get; init; } = [];
    public bool Deleted { get; init; }
    public long Revision { get; init; }
    public Guid MutationId { get; init; } = Guid.NewGuid();
}
public record Change(Entry Value, long BaseRevision);
public record SyncRequest(Guid VaultId, List<Change> Changes);
public record SyncResponse(List<Entry> Entries, List<Guid> Applied, List<Guid> Conflicts);
public record LoginRequest(string Username, string Password);
public record LoginResponse(string Token, Guid VaultId);
public class ServerState
{
    public Guid VaultId { get; set; } = Guid.NewGuid();
    public List<Entry> Entries { get; set; } = [];
}
public class LocalState
{
    public string ServerUrl { get; set; } = "";
    public Guid? VaultId { get; set; }
    public List<Entry> Entries { get; set; } = [];
    public Dictionary<Guid, long> Pending { get; set; } = [];
    public Dictionary<Guid, Entry> Conflicts { get; set; } = [];
    public DateTimeOffset? LastSync { get; set; }
}
public static class SyncEngine
{
    public static void Validate(Entry e)
    {
        if (e.Id == Guid.Empty || e.MutationId == Guid.Empty || e.Revision < 0 ||
            e.Kind is not ("person" or "license" or "recovery") || string.IsNullOrWhiteSpace(e.Name) ||
            e.Name.Length > 200 || e.Code.Length > 10000 || e.Notes.Length > 20000 ||
            e.SoftwareVersion.Length > 200 || e.Installations.Count > 100 ||
            e.Installations.Any(i => i.Platform is not ("Windows" or "Linux" or "macOS") || i.Device.Length > 200))
            throw new ArgumentException("Ongeldige invoer of te lange tekst.");
        if (e.Kind == "recovery") (e.Recovery ?? throw new ArgumentException("Herstelgegevens ontbreken.")).Validate(e.Deleted);
        else if (e.Recovery is not null) throw new ArgumentException("Herstelgegevens horen bij een herstelsleutel.");
        if (e.Kind == "license" && e.OwnerId is null) throw new ArgumentException("Selecteer een gebruiker.");
    }
    // Caller serializes access and saves the returned copy durably before responding.
    public static (ServerState State, SyncResponse Response) Apply(ServerState state, SyncRequest request)
    {
        if (request.VaultId != state.VaultId) throw new ArgumentException("Verkeerde serverkluis.");
        if (request.Changes.Count > 10000 || request.Changes.Select(c => c.Value.Id).Distinct().Count() != request.Changes.Count)
            throw new ArgumentException("Ongeldige batch.");
        foreach (var c in request.Changes) { Validate(c.Value); if (c.BaseRevision < 0) throw new ArgumentException("Ongeldige revisie."); }
        var map = state.Entries.ToDictionary(e => e.Id);
        List<Guid> applied = [], conflicts = [];
        foreach (var change in request.Changes)
        {
            var e = change.Value;
            map.TryGetValue(e.Id, out var current);
            if (current?.MutationId == e.MutationId) { applied.Add(e.Id); continue; }
            if ((current?.Revision ?? 0) != change.BaseRevision || (current is not null && current.Kind != e.Kind))
            { conflicts.Add(e.Id); continue; }
            map[e.Id] = e with { Revision = checked((current?.Revision ?? 0) + 1) };
            applied.Add(e.Id);
        }
        // Tombstoned people stay available for historical references. A live license can be reassigned.
        foreach (var e in map.Values.Where(e => e.Kind == "license" && !e.Deleted))
            if (!map.TryGetValue(e.OwnerId!.Value, out var owner) || owner.Kind != "person")
                throw new ArgumentException("Gebruiker ontbreekt; synchroniseer de gebruiker mee.");
        RecoveryData.ValidateTotal(map.Values);
        var next = new ServerState { VaultId = state.VaultId, Entries = map.Values.ToList() };
        return (next, new SyncResponse(next.Entries, applied, conflicts));
    }
    public static void Edit(LocalState state, Entry entry)
    {
        Validate(entry);
        var old = state.Entries.FirstOrDefault(e => e.Id == entry.Id);
        state.Pending.TryAdd(entry.Id, old?.Revision ?? 0);
        state.Entries.RemoveAll(e => e.Id == entry.Id);
        state.Entries.Add(entry with { MutationId = Guid.NewGuid(), Revision = old?.Revision ?? 0 });
    }
    public static void Merge(LocalState state, SyncResponse response)
    {
        foreach (var id in response.Applied) { state.Pending.Remove(id); state.Conflicts.Remove(id); }
        foreach (var remote in response.Entries)
        {
            if (state.Pending.ContainsKey(remote.Id)) { state.Conflicts[remote.Id] = remote; continue; }
            state.Entries.RemoveAll(e => e.Id == remote.Id);
            state.Entries.Add(remote);
        }
        state.LastSync = DateTimeOffset.Now;
    }
    public static void Resolve(LocalState state, Guid id, bool keepLocal)
    {
        var remote = state.Conflicts[id];
        if (keepLocal)
        {
            var local = state.Entries.Single(e => e.Id == id);
            state.Entries.Remove(local);
            state.Entries.Add(local with { Revision = remote.Revision, MutationId = Guid.NewGuid() });
            state.Pending[id] = remote.Revision;
        }
        else { state.Entries.RemoveAll(e => e.Id == id); state.Entries.Add(remote); state.Pending.Remove(id); }
        state.Conflicts.Remove(id);
    }
}
