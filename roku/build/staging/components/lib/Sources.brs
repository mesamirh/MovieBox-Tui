' Alternative providers, used when MovieBox can't play a title (Task threads only).
' Results from 4KHDHub (below) and Dramachi (Dramachi.brs) are merged by SRC_Search.
'
' 4KHDHub flow (mirrors the TUI provider in src/providers/fourkhdhub):
'   search page -> title page -> releases (file name, size, mirror links)
'   -> GreenMotors redirect (base64 / rot13 payload) -> HubCloud drive page
'   -> resolver page -> direct file links, checked with a small ranged GET.
' HubDrive mirrors need an account, so only HubCloud and direct links are used.

function SRC_Base() as String
    return "https://4khdhub.one"
end function

function SRC_UserAgent() as String
    return "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36"
end function

function SRC_Get(url as String, timeoutMs as Integer) as Object
    headers = [["User-Agent", SRC_UserAgent()], ["Accept", "text/html,application/xhtml+xml,*/*;q=0.8"], ["Accept-Language", "en-US,en;q=0.9"]]
    res = MB_HttpRequest("GET", url, invalid, headers, timeoutMs)
    MB_Log("src GET " + url + " -> " + res.code.ToStr() + " (" + Len(res.body).ToStr() + " bytes)")
    return res
end function

function SRC_Match(pattern as String, text as String) as String
    re = CreateObject("roRegex", pattern, "is")
    groups = re.Match(text)
    if groups.Count() > 1 then return groups[1]
    return ""
end function

function SRC_DecodeEntities(text as String) as String
    out = text.Replace("&amp;", "&").Replace("&#039;", "'").Replace("&#39;", "'").Replace("&quot;", Chr(34))
    out = out.Replace("&nbsp;", " ").Replace("&#8211;", "-").Replace("&#8217;", "'").Replace("&lt;", "<").Replace("&gt;", ">")
    return out.Trim()
end function

function SRC_B64(text as String) as String
    ba = CreateObject("roByteArray")
    ba.FromBase64String(text)
    return ba.ToAsciiString()
end function

function SRC_Rot13(text as String) as String
    out = ""
    for i = 1 to Len(text)
        code = Asc(Mid(text, i, 1))
        if (code >= 65 and code <= 77) or (code >= 97 and code <= 109) then
            code = code + 13
        else if (code >= 78 and code <= 90) or (code >= 110 and code <= 122) then
            code = code - 13
        end if
        out = out + Chr(code)
    end for
    return out
end function

' Direct file URLs often contain the raw file name (spaces, brackets).
function SRC_EncodeUrl(url as String) as String
    return url.Replace(" ", "%20").Replace("[", "%5B").Replace("]", "%5D")
end function

' Every href="https://..." in a block, with the anchor text that follows it.
function SRC_Links(block as String) as Object
    links = []
    needle = "href=" + Chr(34) + "https://"
    at = Instr(1, block, needle)
    while at > 0
        start = at + 6
        finish = Instr(start, block, Chr(34))
        if finish = 0 then exit while
        href = SRC_DecodeEntities(Mid(block, start, finish - start))
        textStart = Instr(finish, block, ">")
        textEnd = Instr(textStart + 1, block, "</a")
        label = ""
        if textStart > 0 and textEnd > textStart then
            raw = Mid(block, textStart + 1, textEnd - textStart - 1)
            label = SRC_DecodeEntities(CreateObject("roRegex", "<[^>]*>", "s").ReplaceAll(raw, " "))
        end if
        links.Push({ href: href, label: label })
        at = Instr(finish, block, needle)
    end while
    return links
end function

' ---------------------------------------------------------------------------
' Release metadata parsed from file names
' ---------------------------------------------------------------------------

function SRC_Height(name as String) as Integer
    lower = LCase(name)
    if Instr(1, lower, "2160p") > 0 or Instr(1, lower, " 4k ") > 0 or Instr(1, lower, ".4k.") > 0 or Instr(1, lower, "uhd") > 0 then return 2160
    if Instr(1, lower, "1080p") > 0 then return 1080
    if Instr(1, lower, "720p") > 0 then return 720
    if Instr(1, lower, "540p") > 0 then return 540
    if Instr(1, lower, "480p") > 0 then return 480
    return 0
end function

function SRC_Codec(name as String) as String
    lower = LCase(name)
    for each token in ["hevc", "x265", "h265", "h.265"]
        if Instr(1, lower, token) > 0 then return "hevc"
    end for
    if Instr(1, lower, "av1") > 0 then return "av1"
    if Instr(1, lower, "vc-1") > 0 or Instr(1, lower, "vc1") > 0 then return "vc1"
    return "h264"
end function

function SRC_HasWord(lower as String, words as Object) as Boolean
    for each w in words
        if Instr(1, lower, w) > 0 then return true
    end for
    return false
end function

' Audio languages named in the file name (Hindi is tracked but never shown).
function SRC_Languages(name as String) as Object
    lower = LCase(name)
    return {
        spa: SRC_HasWord(lower, ["spanish", "latino", "español", "espanol", "castellano", ".spa.", "-spa"])
        eng: SRC_HasWord(lower, ["english", ".eng.", "-eng", "eng-", " eng "])
        hin: SRC_HasWord(lower, ["hindi"])
        other: SRC_HasWord(lower, ["tamil", "telugu", "korean", "japanese", "french", "german", "italian", "chinese", "multi"])
    }
end function

function SRC_ParseEpisode(name as String) as Object
    re = CreateObject("roRegex", "s(\d{1,2})[ ._-]?e(\d{1,3})", "i")
    groups = re.Match(name)
    if groups.Count() > 2 then return { se: groups[1].ToInt(), ep: groups[2].ToInt() }
    return invalid
end function

function SRC_SizeGb(text as String) as Float
    value = SRC_Match("([\d.]+)\s*([GMT]B)", text)
    if value = "" then return 0
    unit = UCase(SRC_Match("[\d.]+\s*([GMT]B)", text))
    gb = value.ToFloat()
    if unit = "MB" then gb = gb / 1024
    if unit = "TB" then gb = gb * 1024
    return gb
end function

' Higher is better: a 1080p H.264 file of moderate size beats a 60 GB 4K remux.
' Files up to 6 GB cost little; past that every GB costs more, so a light 720p file
' beats a 20 GB 1080p one and starts faster on Wi-Fi.
function SRC_Score(rel as Object, caps as Object) as Integer
    score = 200
    if rel.height = 1080 then score = 400
    if rel.height = 720 then score = 320
    if rel.height = 540 then score = 230
    if rel.height = 480 then score = 150
    if rel.height = 2160 then
        if caps.uhd then score = 300 else score = -2000
    end if
    if rel.codec = "h264" then score = score + 30
    if rel.codec = "hevc" and not caps.hevc then score = -5000
    if rel.codec = "vc1" or rel.codec = "av1" then score = -5000
    if Instr(1, LCase(rel.file), "remux") > 0 then score = score - 300
    score = score - Int(rel.sizeGb * 4)
    if rel.sizeGb > 6 then score = score - Int((rel.sizeGb - 6) * 16)
    if rel.langs.spa then score = score + 80
    if rel.langs.eng then score = score + 20
    return score
end function

function SRC_DeviceCaps() as Object
    info = CreateObject("roDeviceInfo")
    hevc = info.CanDecodeVideo({ Codec: "hevc" }).result = true
    mode = info.GetVideoMode()
    return { hevc: hevc, uhd: hevc and Instr(1, mode, "2160") > 0 }
end function

function SRC_ReleaseLabel(rel as Object) as String
    parts = [rel.provider]
    if rel.height > 0 then parts.Push(rel.height.ToStr() + "p")
    if rel.codec = "hevc" then parts.Push("HEVC")
    if rel.codec = "h264" then parts.Push("H.264")
    if rel.size <> "" then parts.Push(rel.size)
    langs = []
    if rel.langs.spa then langs.Push("Spanish")
    if rel.langs.eng then langs.Push("English")
    if langs.Count() > 0 then parts.Push(langs.Join(", "))
    if Instr(1, LCase(rel.file), "remux") > 0 then parts.Push("Remux")
    return parts.Join("  ·  ")
end function

' ---------------------------------------------------------------------------
' Search and releases
' ---------------------------------------------------------------------------

function SRC_FindPage(title as String, year as String, stype as Integer) as String
    escaper = CreateObject("roUrlTransfer")
    res = SRC_Get(SRC_Base() + "/?s=" + escaper.Escape(title), 10000)
    if res.code <> 200 then return ""

    wanted = SUB_Normalize(title)
    bestPath = ""
    bestScore = 0
    index = 0
    for each chunk in res.body.Split("<a href=" + Chr(34))
        if Instr(1, chunk, "class=" + Chr(34) + "movie-card" + Chr(34)) > 0 then
            path = Left(chunk, Instr(1, chunk, Chr(34)) - 1)
            isSeries = Instr(1, path, "-series-") > 0
            if Left(path, 1) = "/" and isSeries = (stype = 2) then
                name = SUB_Normalize(SRC_DecodeEntities(SRC_Match("movie-card-title" + Chr(34) + ">([^<]*)<", chunk)))
                metaYear = SRC_Match("movie-card-meta" + Chr(34) + ">\s*(\d{4})", chunk)
                score = 0
                if name = wanted then
                    score = 10
                else if name <> "" and wanted <> "" and (Instr(1, name, wanted) = 1 or Instr(1, wanted, name) = 1) then
                    score = 3
                end if
                if score > 0 and year <> "" and metaYear <> "" then
                    diff = Abs(metaYear.ToInt() - year.ToInt())
                    if diff = 0 then
                        score = score + 5
                    else if diff = 1 then
                        score = score + 2
                    else if stype <> 2 then
                        ' Same name, different year: a remake or another film.
                        score = score - 6
                    end if
                end if
                score = score * 10 - index
                if score > bestScore then
                    bestScore = score
                    bestPath = path
                end if
                index = index + 1
            end if
        end if
    end for
    MB_Log("src 4khdhub match for " + title + ": " + bestPath)
    return bestPath
end function

function SRC_ParseReleases(html as String, se as Integer, ep as Integer) as Object
    caps = SRC_DeviceCaps()
    q = Chr(34)
    if se > 0 then
        separator = "class=" + q + "episode-download-item" + q
        titlePattern = "episode-file-title" + q + ">\s*([^<]+?)\s*<"
    else
        separator = "class=" + q + "download-item"
        titlePattern = "class=" + q + "file-title" + q + ">\s*([^<]+?)\s*<"
    end if

    releases = []
    seen = {}
    chunks = html.Split(separator)
    for i = 1 to chunks.Count() - 1
        block = chunks[i]
        file = SRC_DecodeEntities(SRC_Match(titlePattern, block))
        lowerFile = LCase(file)
        keep = file <> "" and Right(lowerFile, 4) <> ".zip" and seen[lowerFile] = invalid
        if keep and se > 0 then
            parsed = SRC_ParseEpisode(file)
            keep = parsed <> invalid and parsed.se = se and parsed.ep = ep
        end if
        if keep then
            mirrors = []
            for each link in SRC_Links(block)
                h = LCase(link.href)
                isMirror = Instr(1, h, "greenmotors.") > 0 or Instr(1, h, "greenmountmotors.") > 0 or Instr(1, h, "hubcloud.") > 0 or Instr(1, h, "pixeldrain.") > 0
                if isMirror and Instr(1, LCase(link.label), "hubdrive") = 0 then mirrors.Push(link.href)
            end for
            langs = SRC_Languages(file)
            hindiOnly = langs.hin and not langs.eng and not langs.spa and not langs.other
            if mirrors.Count() > 0 and not hindiOnly then
                seen[lowerFile] = true
                size = SRC_Match(">\s*([\d.]+\s*[GMT]B)\s*<", block)
                rel = {
                    provider: "4KHDHub"
                    file: file
                    size: size
                    sizeGb: SRC_SizeGb(size)
                    height: SRC_Height(file)
                    codec: SRC_Codec(file)
                    langs: langs
                    mirrors: mirrors
                }
                rel.score = SRC_Score(rel, caps)
                rel.label = SRC_ReleaseLabel(rel)
                if rel.score > -1000 then releases.Push(rel)
            end if
        end if
    end for

    return SRC_SortReleases(releases)
end function

' Best first (insertion sort; lists are short).
function SRC_SortReleases(releases as Object) as Object
    for i = 1 to releases.Count() - 1
        current = releases[i]
        j = i - 1
        while j >= 0 and releases[j].score < current.score
            releases[j + 1] = releases[j]
            j = j - 1
        end while
        releases[j + 1] = current
    end for
    return releases
end function

' 4KHDHub releases: { ok, releases, error }.
function SRC_FourKSearch(title as String, year as String, stype as Integer, se as Integer, ep as Integer) as Object
    path = SRC_FindPage(title, year, stype)
    if path = "" then return { ok: false, error: "No other provider has this title.", releases: [] }
    res = SRC_Get(SRC_Base() + path, 12000)
    if res.code <> 200 then return { ok: false, error: "The other providers are not responding.", releases: [] }
    releases = SRC_ParseReleases(res.body, se, ep)
    if releases.Count() = 0 then
        msg = "No compatible files were found on other providers."
        if se > 0 then msg = "No compatible files for " + EpisodeCode(se, ep) + " on other providers."
        return { ok: false, error: msg, releases: [] }
    end if
    return { ok: true, releases: releases }
end function

' All enabled providers (4KHDHub, Dramachi and Stremio addons) merged and ranked: { ok, releases, error }.
function SRC_Search(title as String, year as String, stype as Integer, se as Integer, ep as Integer) as Object
    releases = []
    message = "No enabled provider has this title. Manage providers in the Providers tab."
    if ProviderEnabled("4khdhub") then
        fourk = SRC_FourKSearch(title, year, stype, se, ep)
        releases.Append(fourk.releases)
        message = fourk.error
    end if
    if ProviderEnabled("dramachi") then releases.Append(DRC_Search(title, year, stype, se, ep))
    releases.Append(ADD_Search(title, year, stype, se, ep))
    if releases.Count() = 0 then return { ok: false, error: message, releases: [] }
    releases = SRC_SortReleases(releases)
    MB_Log("src releases: " + releases.Count().ToStr() + ", best " + releases[0].label)
    return { ok: true, releases: releases }
end function

' ---------------------------------------------------------------------------
' Mirror resolution
' ---------------------------------------------------------------------------

function SRC_GreenMotors(url as String) as String
    res = SRC_Get(url, 8000)
    if res.code <> 200 then return ""
    payload = SRC_Match("s\(\s*['" + Chr(34) + "]o['" + Chr(34) + "]\s*,\s*['" + Chr(34) + "]([^'" + Chr(34) + "]+)['" + Chr(34) + "]", res.body)
    if payload = "" then return ""
    json = ParseJson(SRC_B64(SRC_Rot13(SRC_B64(SRC_B64(payload)))))
    if json = invalid or json.o = invalid then return ""
    return SRC_B64(MB_Str(json.o))
end function

function SRC_PixeldrainApi(url as String) as String
    id = SRC_Match("pixeldrain\.[a-z]+/(?:u|api/file)/([A-Za-z0-9_-]+)", url)
    host = SRC_Match("https://(pixeldrain\.[a-z]+)/", url)
    if id = "" or host = "" then return ""
    return "https://" + host + "/api/file/" + id + "?download"
end function

' Direct candidates from a HubCloud drive page, best first.
function SRC_HubCloud(driveUrl as String) as Object
    res = SRC_Get(driveUrl, 8000)
    if res.code <> 200 then return []
    q = Chr(34)
    resolver = SRC_Match("href=" + q + "(https://[^" + q + "]*(?:gamerxyt\.com|hubcloud\.php)[^" + q + "]*)" + q, res.body)
    if resolver = "" then resolver = SRC_Match("id=" + q + "download" + q + "[^>]*href=" + q + "(https://[^" + q + "]+)" + q, res.body)
    if resolver = "" then return []
    page = SRC_Get(SRC_DecodeEntities(resolver), 10000)
    if page.code <> 200 then return []

    first = []
    second = []
    last = []
    for each link in SRC_Links(page.body)
        h = LCase(link.href)
        pixel = SRC_PixeldrainApi(link.href)
        if pixel <> "" then
            second.Push(pixel)
        else if Instr(1, h, ".workers.dev/") > 0 and Instr(1, h, "pixel.") = 0 and Instr(1, h, "gpdl.") = 0 then
            first.Push(SRC_EncodeUrl(link.href))
        else if Instr(1, h, ".r2.dev/") > 0 or Instr(1, h, "googleusercontent.com/") > 0 or Instr(1, h, "storage.googleapis.com/") > 0 then
            second.Push(SRC_EncodeUrl(link.href))
        else if Instr(1, h, "pages.dev") > 0 then
            wrapped = SRC_B64(SRC_Match("[?&]u=([^&]+)", link.href))
            if Left(wrapped, 8) = "https://" then second.Push(SRC_EncodeUrl(wrapped))
        else if Instr(1, h, "hubcloud.") > 0 and Instr(1, h, "?id=") > 0 and Instr(1, h, "/drive/") = 0 then
            ' "10Gbps" servers: redirect chains that sometimes end on a wrapper page.
            last.Push(link.href)
        end if
    end for
    ' PixelDrain links also appear inside scripts, not only as anchors.
    for each prefix in ["https://pixeldrain.dev/", "https://pixeldrain.com/"]
        at = Instr(1, page.body, prefix)
        while at > 0
            api = SRC_PixeldrainApi(Mid(page.body, at, 80))
            if api <> "" then second.Push(api)
            at = Instr(at + Len(prefix), page.body, prefix)
        end while
    end for

    out = []
    seen = {}
    for each group in [first, second, last]
        for each url in group
            if seen[url] = invalid then
                seen[url] = true
                out.Push(url)
            end if
        end for
    end for
    return out
end function

' True when the URL answers like a media file (not an HTML error page).
function SRC_Probe(url as String) as Boolean
    return SRC_ProbeWith(url, [], false)
end function

' Playlists (HLS/DASH) are text, so allowText accepts any successful answer.
function SRC_ProbeWith(url as String, headers as Object, allowText as Boolean) as Boolean
    port = CreateObject("roMessagePort")
    xfer = CreateObject("roUrlTransfer")
    xfer.SetMessagePort(port)
    xfer.SetUrl(url)
    xfer.SetCertificatesFile("common:/certs/ca-bundle.crt")
    xfer.InitClientCertificates()
    xfer.AddHeader("User-Agent", SRC_UserAgent())
    for each h in headers
        xfer.AddHeader(h[0], h[1])
    end for
    xfer.AddHeader("Range", "bytes=0-2047")
    if not xfer.AsyncGetToString() then return false
    msg = Wait(7000, port)
    if Type(msg) <> "roUrlEvent" then
        xfer.AsyncCancel()
        MB_Log("src probe timeout " + url)
        return false
    end if
    code = msg.GetResponseCode()
    contentType = ""
    headers = msg.GetResponseHeaders()
    if headers <> invalid then
        for each key in headers
            if LCase(key) = "content-type" then contentType = LCase(headers[key])
        end for
    end if
    MB_Log("src probe " + code.ToStr() + " " + contentType + " " + Left(url, 90))
    if code <> 206 and code <> 200 then return false
    if allowText then return true
    if Instr(1, contentType, "text/") > 0 or Instr(1, contentType, "html") > 0 or Instr(1, contentType, "zip") > 0 then return false
    return true
end function

' Reads the first 8 KB of a file and returns its video codec from the container header:
' "hevc", "h264", "av1", "vc1", "unknown" (reachable but not recognised) or "" (not reachable).
function SRC_SniffCodec(url as String) as String
    path = "tmp:/src_sniff.bin"
    fs = CreateObject("roFileSystem")
    if fs.Exists(path) then fs.Delete(path)
    port = CreateObject("roMessagePort")
    xfer = CreateObject("roUrlTransfer")
    xfer.SetMessagePort(port)
    xfer.SetUrl(url)
    xfer.SetCertificatesFile("common:/certs/ca-bundle.crt")
    xfer.InitClientCertificates()
    xfer.AddHeader("User-Agent", SRC_UserAgent())
    xfer.AddHeader("Range", "bytes=0-8191")
    if not xfer.AsyncGetToFile(path) then return ""
    msg = Wait(7000, port)
    if Type(msg) <> "roUrlEvent" then
        xfer.AsyncCancel()
        MB_Log("src sniff timeout " + url)
        return ""
    end if
    code = msg.GetResponseCode()
    if code <> 206 and code <> 200 then
        MB_Log("src sniff " + code.ToStr() + " " + Left(url, 90))
        return ""
    end if
    bytes = CreateObject("roByteArray")
    bytes.ReadFile(path)
    fs.Delete(path)
    codec = "unknown"
    if SRC_BytesHave(bytes, "V_MPEGH") or SRC_BytesHave(bytes, "hvc1") or SRC_BytesHave(bytes, "hev1") then
        codec = "hevc"
    else if SRC_BytesHave(bytes, "V_MPEG4/ISO/AVC") or SRC_BytesHave(bytes, "avc1") then
        codec = "h264"
    else if SRC_BytesHave(bytes, "V_AV1") or SRC_BytesHave(bytes, "av01") then
        codec = "av1"
    else if SRC_BytesHave(bytes, "V_MS/VFW") then
        codec = "vc1"
    end if
    MB_Log("src sniff " + codec + " " + Left(url, 90))
    return codec
end function

' Whether an ASCII marker occurs anywhere in a byte array (binary safe).
function SRC_BytesHave(bytes as Object, marker as String) as Boolean
    first = Asc(marker)
    size = Len(marker)
    last = bytes.Count() - size
    for i = 0 to last
        if bytes[i] = first then
            same = true
            for k = 1 to size - 1
                if bytes[i + k] <> Asc(Mid(marker, k + 1, 1)) then
                    same = false
                    exit for
                end if
            end for
            if same then return true
        end if
    end for
    return false
end function

function SRC_StreamFormat(url as String) as String
    lower = LCase(url)
    if Instr(1, lower, ".mp4") > 0 then return "mp4"
    return "mkv"
end function

' Resolves one release to a playable source, or invalid.
function SRC_Resolve(rel as Object) as Dynamic
    if rel.url <> invalid and rel.url <> "" then
        ' Direct file (Dramachi, addons): just check that it still answers.
        kind = "mkv"
        if rel.kind <> invalid then
            kind = rel.kind
        else if rel.ext = "mp4" then
            kind = "mp4"
        end if
        headers = []
        if rel.headers <> invalid then headers = rel.headers
        if not SRC_ProbeWith(rel.url, headers, kind = "hls" or kind = "dash") then return invalid
        return SRC_Source(rel, rel.url, kind)
    end if
    tried = 0
    for each mirror in rel.mirrors
        target = mirror
        lower = LCase(mirror)
        if Instr(1, lower, "greenmotors.") > 0 or Instr(1, lower, "greenmountmotors.") > 0 then target = SRC_GreenMotors(mirror)
        candidates = []
        lowerTarget = LCase(target)
        if Instr(1, lowerTarget, "hubcloud.") > 0 and Instr(1, lowerTarget, "/drive/") > 0 then
            candidates = SRC_HubCloud(target)
        else if Instr(1, lowerTarget, "pixeldrain.") > 0 then
            candidates = [SRC_PixeldrainApi(target)]
        end if
        MB_Log("src mirror " + Left(target, 80) + " -> " + candidates.Count().ToStr() + " candidates")
        probes = 0
        for each url in candidates
            if url <> "" and probes < 5 then
                probes = probes + 1
                if SRC_Probe(url) then return SRC_Source(rel, url, SRC_StreamFormat(url))
            end if
        end for
        tried = tried + 1
        if tried >= 3 then exit for
    end for
    return invalid
end function

function SRC_Source(rel as Object, url as String, kind as String) as Object
    return {
        kind: kind
        url: url
        resolution: rel.height
        codec: rel.codec
        size: rel.size
        resourceId: ""
        cookie: ""
        userAgent: SRC_UserAgent()
        headers: SRC_SourceHeaders(rel)
        provider: rel.provider
        label: rel.label
    }
end function

function SRC_SourceHeaders(rel as Object) as Object
    if rel.headers = invalid then return []
    return rel.headers
end function

' Search plus resolve of the best releases; returns { ok, source, releases, error }.
function SRC_Auto(title as String, year as String, stype as Integer, se as Integer, ep as Integer) as Object
    found = SRC_Search(title, year, stype, se, ep)
    if found.ok <> true then return found
    ' Up to three files per provider, best first, so one provider being down
    ' doesn't hide the other.
    attempts = {}
    for each rel in found.releases
        count = attempts[rel.provider]
        if count = invalid then count = 0
        if count < 3 then
            attempts[rel.provider] = count + 1
            src = SRC_Resolve(rel)
            if src <> invalid then return { ok: true, source: src, releases: found.releases }
        end if
    end for
    return { ok: false, error: "Other providers have this title, but no mirror answered.", releases: found.releases }
end function
