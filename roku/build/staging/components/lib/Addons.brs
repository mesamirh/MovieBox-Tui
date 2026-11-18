' Stremio addons as providers (Task threads only), like the TUI (src/providers/addons).
'
' Any addon that serves the Stremio "stream" resource for IMDb ids can be added by its
' manifest URL. The title is resolved to an IMDb id through Cinemeta, every enabled addon is
' asked in parallel for /stream/<type>/<id>.json, and streams with a direct http(s) URL become
' releases ranked with the same rules as the built-in providers. Torrent-only results
' (infoHash) can't play on a Roku and are skipped; debrid-configured addons return HTTP links.

' Accepts "stremio://host/path", a bare host, or a URL without "/manifest.json".
function ADD_NormalizeUrl(text as String) as String
    url = text.Trim()
    if url = "" then return ""
    if LCase(Left(url, 10)) = "stremio://" then url = "https://" + Mid(url, 11)
    if LCase(Left(url, 7)) <> "http://" and LCase(Left(url, 8)) <> "https://" then url = "https://" + url
    if LCase(Right(url, 14)) <> "/manifest.json" then
        while Right(url, 1) = "/"
            url = Left(url, Len(url) - 1)
        end while
        url = url + "/manifest.json"
    end if
    return url
end function

function ADD_Base(manifestUrl as String) as String
    return Left(manifestUrl, Len(manifestUrl) - Len("/manifest.json"))
end function

' Fetches and checks a manifest: { ok, addon, error }.
function ADD_FetchManifest(text as String) as Object
    url = ADD_NormalizeUrl(text)
    if url = "" then return { ok: false, error: "Type the addon URL first." }
    res = MB_HttpRequest("GET", url, invalid, [["User-Agent", SRC_UserAgent()], ["Accept", "application/json"]], 10000)
    MB_Log("addon manifest " + url + " -> " + res.code.ToStr())
    if res.code < 200 or res.code >= 300 then return { ok: false, error: "That URL didn't answer (" + res.code.ToStr() + "). Check it and try again." }
    manifest = ParseJson(res.body)
    if manifest = invalid or Type(manifest) <> "roAssociativeArray" or MB_Str(manifest.id) = "" then
        return { ok: false, error: "That URL is not a Stremio addon manifest." }
    end if

    hints = manifest.behaviorHints
    if hints <> invalid and hints.configurationRequired = true then
        return { ok: false, error: "This addon must be configured first. Open its configure page on a computer or phone and paste the URL it gives you." }
    end if

    hasStream = false
    prefixes = ADD_StrList(manifest.idPrefixes)
    types = ADD_StrList(manifest.types)
    if manifest.resources <> invalid then
        for each r in manifest.resources
            if Type(r) = "roString" or Type(r) = "String" then
                if r = "stream" then hasStream = true
            else if Type(r) = "roAssociativeArray" and MB_Str(r.name) = "stream" then
                hasStream = true
                if r.idPrefixes <> invalid then prefixes = ADD_StrList(r.idPrefixes)
                if r.types <> invalid then types = ADD_StrList(r.types)
            end if
        end for
    end if
    if not hasStream then return { ok: false, error: "This addon has no streams (it only adds catalogs or info)." }

    usable = prefixes.Count() = 0
    for each p in prefixes
        if Left(p, 2) = "tt" then usable = true
    end for
    if not usable then return { ok: false, error: "This addon doesn't use IMDb ids, so it can't be searched from here." }
    if not ADD_Has(types, "movie") and not ADD_Has(types, "series") then
        return { ok: false, error: "This addon has no movies or series." }
    end if

    name = MB_Str(manifest.name)
    if name = "" then name = MB_Str(manifest.id)
    if Len(name) > 24 then name = Left(name, 24)
    return { ok: true, addon: { url: url, base: ADD_Base(url), name: name, enabled: true, types: types, prefixes: prefixes } }
end function

function ADD_StrList(value as Dynamic) as Object
    out = []
    if value = invalid or Type(value) <> "roArray" then return out
    for each v in value
        s = MB_Str(v)
        if s <> "" then out.Push(s)
    end for
    return out
end function

function ADD_Has(list as Object, value as String) as Boolean
    for each item in list
        if item = value then return true
    end for
    return false
end function

' GETs several JSON URLs at once; returns one parsed body (or invalid) per URL.
function ADD_FetchAll(urls as Object, timeoutMs as Integer) as Object
    port = CreateObject("roMessagePort")
    transfers = {}
    results = []
    for i = 0 to urls.Count() - 1
        results.Push(invalid)
        xfer = CreateObject("roUrlTransfer")
        xfer.SetMessagePort(port)
        xfer.SetUrl(urls[i])
        xfer.SetCertificatesFile("common:/certs/ca-bundle.crt")
        xfer.InitClientCertificates()
        xfer.EnableEncodings(true)
        xfer.AddHeader("User-Agent", SRC_UserAgent())
        xfer.AddHeader("Accept", "application/json")
        if xfer.AsyncGetToString() then transfers[xfer.GetIdentity().ToStr()] = { xfer: xfer, index: i }
    end for
    pending = transfers.Count()
    deadline = CreateObject("roTimespan")
    while pending > 0 and deadline.TotalMilliseconds() < timeoutMs
        msg = Wait(timeoutMs - deadline.TotalMilliseconds(), port)
        if Type(msg) = "roUrlEvent" then
            entry = transfers[msg.GetSourceIdentity().ToStr()]
            if entry <> invalid and results[entry.index] = invalid then
                pending = pending - 1
                MB_Log("addon GET " + urls[entry.index] + " -> " + msg.GetResponseCode().ToStr())
                if msg.GetResponseCode() = 200 then
                    json = ParseJson(msg.GetString())
                    if json <> invalid then
                        results[entry.index] = json
                    else
                        results[entry.index] = {}
                    end if
                else
                    results[entry.index] = {}
                end if
            end if
        end if
    end while
    for each key in transfers
        transfers[key].xfer.AsyncCancel()
    end for
    return results
end function

' Spanish also shows up as flags in addon titles.
function ADD_Languages(text as String) as Object
    langs = SRC_Languages(text)
    if Instr(1, text, "🇪🇸") > 0 or Instr(1, text, "🇲🇽") > 0 then langs.spa = true
    if Instr(1, text, "🇬🇧") > 0 or Instr(1, text, "🇺🇸") > 0 then langs.eng = true
    if Instr(1, text, "🇮🇳") > 0 then langs.hin = true
    return langs
end function

function ADD_Kind(url as String, text as String) as String
    lower = LCase(url)
    if Instr(1, lower, ".m3u8") > 0 then return "hls"
    if Instr(1, lower, ".mpd") > 0 then return "dash"
    if Instr(1, lower, ".mp4") > 0 then return "mp4"
    if Instr(1, lower, ".mkv") > 0 then return "mkv"
    lowerText = LCase(text)
    if Instr(1, lowerText, ".mp4") > 0 then return "mp4"
    if Instr(1, lowerText, "m3u8") > 0 or Instr(1, lowerText, " hls") > 0 then return "hls"
    return "mkv"
end function

' Turns one addon stream into a release, or invalid when it can't play here.
function ADD_Release(addon as Object, s as Object, caps as Object) as Dynamic
    if Type(s) <> "roAssociativeArray" then return invalid
    url = MB_Str(s.url)
    lowerUrl = LCase(url)
    if Left(lowerUrl, 7) <> "http://" and Left(lowerUrl, 8) <> "https://" then return invalid

    filename = ""
    headers = []
    hints = s.behaviorHints
    if hints <> invalid and Type(hints) = "roAssociativeArray" then
        filename = MB_Str(hints.filename)
        proxy = hints.proxyHeaders
        if proxy <> invalid and proxy.request <> invalid then
            for each key in proxy.request
                headers.Push([key, MB_Str(proxy.request[key])])
            end for
        end if
    end if
    title = MB_Str(s.title)
    if title = "" then title = MB_Str(s.description)
    text = (MB_Str(s.name) + " " + title + " " + filename).Replace(Chr(10), " ")

    file = filename
    if file = "" then file = title.Split(Chr(10))[0]
    if file = "" then file = MB_Str(s.name).Replace(Chr(10), " ")

    langs = ADD_Languages(text)
    if langs.hin and not langs.eng and not langs.spa and not langs.other then return invalid

    size = SRC_Match("([\d.]+\s*[GMT]B)", text)
    rel = {
        provider: addon.name
        file: file
        size: size
        sizeGb: SRC_SizeGb(size)
        height: SRC_Height(text)
        codec: SRC_Codec(text)
        langs: langs
        mirrors: []
        url: url
        kind: ADD_Kind(url, text)
        headers: headers
    }
    if rel.height = 0 and Instr(1, LCase(text), "4k") > 0 then rel.height = 2160
    rel.score = SRC_Score(rel, caps)
    rel.label = SRC_ReleaseLabel(rel)
    if rel.score <= -1000 then return invalid
    return rel
end function

' Releases from every enabled addon (best 10 per addon), or [] when none answer.
function ADD_Search(title as String, year as String, stype as Integer, se as Integer, ep as Integer) as Object
    releases = []
    kind = "movie"
    if stype = 2 then kind = "series"
    addons = []
    for each a in AddonsAll()
        if a.enabled = true and (a.types = invalid or a.types.Count() = 0 or ADD_Has(a.types, kind)) then addons.Push(a)
    end for
    if addons.Count() = 0 then return releases

    imdb = SUB_FindImdbId(title, year, kind)
    if imdb = "" then return releases
    id = imdb
    if kind = "series" then id = imdb + ":" + se.ToStr() + ":" + ep.ToStr()

    urls = []
    for each a in addons
        urls.Push(a.base + "/stream/" + kind + "/" + id + ".json")
    end for
    answers = ADD_FetchAll(urls, 15000)

    caps = SRC_DeviceCaps()
    for i = 0 to addons.Count() - 1
        json = answers[i]
        found = []
        if json <> invalid and json.streams <> invalid and Type(json.streams) = "roArray" then
            for each s in json.streams
                rel = ADD_Release(addons[i], s, caps)
                if rel <> invalid then found.Push(rel)
            end for
        end if
        found = SRC_SortReleases(found)
        for j = 0 to found.Count() - 1
            if j < 10 then releases.Push(found[j])
        end for
        MB_Log("addon " + addons[i].name + ": " + found.Count().ToStr() + " playable streams")
    end for
    return releases
end function
