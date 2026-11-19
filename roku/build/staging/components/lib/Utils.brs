' Shared render-thread helpers: theme, formatting and local library storage.

function Theme() as Object
    return {
        bg: "0x0A0E14FF"
        surface: "0x161C26FF"
        surfaceHi: "0x222A37FF"
        accent: "0xF5B82EFF"
        accentText: "0x0A0E14FF"
        text: "0xF2F4F8FF"
        muted: "0x8A94A6FF"
        dim: "0x5A6476FF"
        danger: "0xFF6B6BFF"
    }
end function

function FormatClock(totalSeconds as Integer) as String
    if totalSeconds < 0 then totalSeconds = 0
    h = Int(totalSeconds / 3600)
    mins = Int((totalSeconds mod 3600) / 60)
    s = totalSeconds mod 60
    if h > 0 then return h.ToStr() + ":" + Right("0" + mins.ToStr(), 2) + ":" + Right("0" + s.ToStr(), 2)
    return mins.ToStr() + ":" + Right("0" + s.ToStr(), 2)
end function

function EpisodeCode(se as Integer, ep as Integer) as String
    return "S" + Right("0" + se.ToStr(), 2) + " E" + Right("0" + ep.ToStr(), 2)
end function

function JoinNonEmpty(parts as Object, sep as String) as String
    out = []
    for each p in parts
        if p <> invalid and p <> "" then out.Push(p)
    end for
    return out.Join(sep)
end function

function MetaLine(node as Object) as String
    kind = "Movie"
    if node.stype = 2 then kind = "Series"
    rating = ""
    if node.rating <> invalid and node.rating <> "" then rating = "IMDb " + node.rating
    return JoinNonEmpty([node.year, kind, node.durationText, node.genre, rating], "   •   ")
end function

' ---------------------------------------------------------------------------
' Registry storage
' ---------------------------------------------------------------------------

function Store() as Object
    return CreateObject("roRegistrySection", "library")
end function

function StoreReadArray(key as String) as Object
    reg = Store()
    if reg.Exists(key) then
        data = ParseJson(reg.Read(key))
        if data <> invalid and Type(data) = "roArray" then return data
    end if
    return []
end function

' flush=false keeps the write in memory. Callers that save several times a
' minute (playback progress) flush only on pause, finish or exit: the registry
' is flash-backed, so a flush per position tick is needless wear.
sub StoreWriteArray(key as String, data as Object, flush as Boolean)
    reg = Store()
    reg.Write(key, FormatJson(data))
    if flush then reg.Flush()
end sub

function PrefGet(name as String, fallback as String) as String
    reg = Store()
    if reg.Exists("pref_" + name) then return reg.Read("pref_" + name)
    return fallback
end function

sub PrefSet(name as String, value as String)
    reg = Store()
    reg.Write("pref_" + name, value)
    reg.Flush()
end sub

' Serialises the fields we keep for a title (from a ContentNode or details AA).
function SnapshotItem(src as Object) as Object
    return {
        id: src.sid
        stype: src.stype
        title: src.title
        tag: src.tag
        year: src.year
        genre: src.genre
        rating: src.rating
        duration: src.durationText
        description: src.description
        poster: src.HDPosterUrl
        backdrop: src.backdrop
    }
end function

function NodeFromSnapshot(item as Object) as Object
    node = CreateObject("roSGNode", "ContentNode")
    node.title = item.title
    node.description = item.description
    node.HDPosterUrl = item.poster
    node.addFields({
        sid: item.id
        stype: item.stype
        tag: item.tag
        year: item.year
        genre: item.genre
        rating: item.rating
        durationText: item.duration
        country: ""
        backdrop: item.backdrop
        wide: ""
        progress: 0.0
        se: 0
        ep: 0
        resumeAt: 0
        subtitle: ""
    })
    return node
end function

' History entries: { item, se, ep, position, duration, updated }
function HistoryAll() as Object
    return StoreReadArray("history")
end function

function HistoryFind(id as String) as Dynamic
    for each entry in HistoryAll()
        if entry.item <> invalid and entry.item.id = id then return entry
    end for
    return invalid
end function

sub HistorySave(item as Object, se as Integer, ep as Integer, position as Integer, duration as Integer, flush as Boolean)
    entries = HistoryAll()
    kept = []
    for each entry in entries
        if entry.item <> invalid and entry.item.id <> item.id then kept.Push(entry)
    end for
    episodes = {}
    previous = HistoryFindIn(entries, item.id)
    if previous <> invalid and previous.episodes <> invalid then episodes = previous.episodes
    if se > 0 or ep > 0 then
        episodes[se.ToStr() + "x" + ep.ToStr()] = { p: position, d: duration }
    end if
    fresh = {
        item: item
        se: se
        ep: ep
        position: position
        duration: duration
        updated: CreateObject("roDateTime").AsSeconds()
        episodes: episodes
    }
    result = [fresh]
    for each entry in kept
        if result.Count() < 40 then result.Push(entry)
    end for
    StoreWriteArray("history", result, flush)
end sub

function HistoryFindIn(entries as Object, id as String) as Dynamic
    for each entry in entries
        if entry.item <> invalid and entry.item.id = id then return entry
    end for
    return invalid
end function

sub HistoryRemove(id as String)
    kept = []
    for each entry in HistoryAll()
        if entry.item <> invalid and entry.item.id <> id then kept.Push(entry)
    end for
    StoreWriteArray("history", kept, true)
end sub

function FavoritesAll() as Object
    return StoreReadArray("favorites")
end function

function FavoriteHas(id as String) as Boolean
    for each item in FavoritesAll()
        if item.id = id then return true
    end for
    return false
end function

' Returns the new state (true = now in My List).
function FavoriteToggle(item as Object) as Boolean
    items = FavoritesAll()
    kept = []
    found = false
    for each fav in items
        if fav.id = item.id then
            found = true
        else
            kept.Push(fav)
        end if
    end for
    if found then
        StoreWriteArray("favorites", kept, true)
        return false
    end if
    result = [item]
    for each fav in kept
        if result.Count() < 100 then result.Push(fav)
    end for
    StoreWriteArray("favorites", result, true)
    return true
end function

' Whether a history entry should appear in Continue Watching.
function HistoryInProgress(entry as Object) as Boolean
    if entry.duration = invalid or entry.duration <= 0 then return entry.position > 30
    ratio = entry.position / entry.duration
    if entry.item.stype = 2 then return entry.position > 30
    return entry.position > 30 and ratio < 0.94
end function

' Mirrors the API-side filter for titles already saved on the device.
' Hiding Hindi releases is the shipped default but it is a preference now:
' the Settings screen turns it off for anyone who wants the whole catalogue.
function HideHindiActive() as Boolean
    return PrefGet("hideHindi", "on") <> "off"
end function

function IsHiddenItem(item as Dynamic) as Boolean
    if item = invalid or item.tag = invalid then return false
    if not HideHindiActive() then return false
    return Instr(1, LCase(item.tag), "hindi") > 0
end function

' Normalises subtitle language codes (ISO 639-1/2 and provider variants) to one key.
function SubLangKey(code as String) as String
    c = LCase(code.Trim())
    for each spanish in ["es", "spa", "spn", "ea", "esla", "es-419", "es-mx", "es-es"]
        if c = spanish then return "spa"
    end for
    if c = "en" or c = "eng" then return "eng"
    if c = "pt" or c = "por" then return "por"
    if c = "pb" or c = "pob" or c = "ptbr" or c = "pt-br" then return "pob"
    if c = "fr" or c = "fre" or c = "fra" then return "fre"
    if c = "de" or c = "ger" or c = "deu" then return "ger"
    if c = "it" or c = "ita" then return "ita"
    if c = "ja" or c = "jpn" then return "jpn"
    if c = "ko" or c = "kor" then return "kor"
    if c = "zh" or c = "chi" or c = "zho" then return "chi"
    if c = "ar" or c = "ara" then return "ara"
    if c = "ru" or c = "rus" then return "rus"
    if c = "tr" or c = "tur" then return "tur"
    if c = "id" or c = "in" or c = "in_id" or c = "ind" then return "ind"
    if c = "hi" or c = "hin" then return "hin"
    lookup = { bn: "ben", fil: "fil", tl: "fil", ha: "hau", ms: "may", pa: "pan", sw: "swa", ur: "urd", vi: "vie", th: "tha", pl: "pol", nl: "dut", sv: "swe", ro: "rum", el: "gre", he: "heb", fa: "per", uk: "ukr", cs: "cze", hu: "hun", ta: "tam", te: "tel" }
    if lookup[c] <> invalid then return lookup[c]
    return c
end function

function SubLangName(key as String) as String
    names = {
        spa: "Español", eng: "English", por: "Português", pob: "Português (Brasil)",
        fre: "Français", ger: "Deutsch", ita: "Italiano", jpn: "Japanese", kor: "Korean",
        chi: "Chinese", ara: "Arabic", rus: "Russian", tur: "Türkçe", ind: "Indonesian",
        hin: "Hindi", ben: "Bengali", fil: "Filipino", hau: "Hausa", may: "Malay", pan: "Punjabi",
        swa: "Kiswahili", urd: "Urdu", vie: "Tiếng Việt", tha: "Thai", pol: "Polski", dut: "Nederlands",
        swe: "Svenska", rum: "Română", gre: "Greek", heb: "Hebrew", per: "Persian", ukr: "Ukrainian",
        cze: "Čeština", hun: "Magyar", tam: "Tamil", tel: "Telugu"
    }
    if names[key] <> invalid then return names[key]
    return UCase(key)
end function

' ---------------------------------------------------------------------------
' Providers: built-in scrapers can be turned off; Stremio addons are added by URL.
' Addon entries: { url (manifest), base, name, enabled, types, prefixes }
' ---------------------------------------------------------------------------

function ProviderEnabled(key as String) as Boolean
    return PrefGet("provider_" + key, "on") = "on"
end function

sub ProviderSetEnabled(key as String, enabled as Boolean)
    value = "off"
    if enabled then value = "on"
    PrefSet("provider_" + key, value)
end sub

function AddonsAll() as Object
    return StoreReadArray("addons")
end function

sub AddonsSave(addons as Object)
    StoreWriteArray("addons", addons, true)
end sub

' Adds or replaces (same manifest URL) an addon; returns false when it was already there.
function AddonsAdd(addon as Object) as Boolean
    kept = []
    replaced = false
    for each a in AddonsAll()
        if a.url = addon.url then
            replaced = true
        else
            kept.Push(a)
        end if
    end for
    kept.Push(addon)
    AddonsSave(kept)
    return not replaced
end function

sub AddonsRemove(url as String)
    kept = []
    for each a in AddonsAll()
        if a.url <> url then kept.Push(a)
    end for
    AddonsSave(kept)
end sub

sub AddonsSetEnabled(url as String, enabled as Boolean)
    addons = AddonsAll()
    for each a in addons
        if a.url = url then a.enabled = enabled
    end for
    AddonsSave(addons)
end sub
