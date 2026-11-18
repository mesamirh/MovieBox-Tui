' Dramachi provider (Task threads only), ported from the TUI (src/providers/dramachi).
'
' A public JSON API with Hollywood movies, TV series and Asian dramas:
'   search -> title_v2 (seasons and versions) -> eplist (files) -> getFile (CDN host + path).
' Files are small 540p-1080p encodes (usually HEVC), served directly from the CDN,
' so they start fast on slow connections. Each file is sniffed to learn its real codec.

function DRC_Base() as String
    return "https://api.nodeobjects.com/"
end function

function DRC_Get(query as String) as Dynamic
    res = MB_HttpRequest("GET", DRC_Base() + "?" + query, invalid, [["User-Agent", SRC_UserAgent()], ["Accept", "application/json"]], 8000)
    MB_Log("drc GET " + query + " -> " + res.code.ToStr())
    if res.code < 200 or res.code >= 300 then return invalid
    return ParseJson(res.body)
end function

' Title without a trailing "(2019)" or " 2019".
function DRC_CleanTitle(text as String) as String
    re = CreateObject("roRegex", "\s*\(?\d{4}\)?\s*$", "")
    return re.ReplaceAll(text, "")
end function

function DRC_IsHiddenCategory(category as String) as Boolean
    lower = LCase(category)
    for each word in ["bollywood", "hindi", "punjabi", "tamil", "telugu", "south"]
        if Instr(1, lower, word) > 0 then return true
    end for
    return false
end function

' Returns { id, category } of the matching title, or invalid.
function DRC_FindTitle(title as String, year as String, stype as Integer) as Dynamic
    escaper = CreateObject("roUrlTransfer")
    json = DRC_Get("interface=search&q=" + escaper.Escape(title) + "&filter=all&page=1")
    if json = invalid or json.data = invalid then return invalid

    wanted = SUB_Normalize(title)
    best = invalid
    bestScore = 0
    for each item in json.data
        category = MB_Str(item.category)
        isMovie = LCase(MB_Str(item.content)) = "movies" or LCase(MB_Str(item.content)) = "movie"
        if (stype = 2) <> isMovie and not DRC_IsHiddenCategory(category) then
            score = 0
            names = [SUB_Normalize(DRC_CleanTitle(MB_Str(item.common_title))), SUB_Normalize(DRC_CleanTitle(MB_Str(item.title)))]
            for each name in names
                if name = wanted then score = 4
            end for
            ' Series years on Dramachi are unreliable, so only movies use them.
            if score > 0 and stype <> 2 and year <> "" then
                if MB_Str(item.year) = year then
                    score = score + 3
                else
                    score = score - 2
                end if
            end if
            if score > bestScore then
                bestScore = score
                best = { id: MB_Str(item.id), category: category }
            end if
        end if
    end for
    return best
end function

' Picks the version to play: the original audio first, then an (English) dub, never Hindi.
function DRC_PickRip(details as Object, stype as Integer, se as Integer) as Dynamic
    seasons = details.seasons
    if seasons = invalid then return invalid
    group = invalid
    if stype = 2 then
        group = seasons["Season " + Right("0" + se.ToStr(), 2)]
    else
        for each key in seasons
            if group = invalid then group = seasons[key]
        end for
    end if
    if group = invalid or group.versions = invalid then return invalid
    fallback = invalid
    for each version in group.versions
        name = LCase(MB_Str(version.version_name))
        rip = MB_Str(version.rip)
        hindi = Instr(1, name, "hindi") > 0 or Instr(1, " " + LCase(rip) + " ", " hi ") > 0
        if rip <> "" and not hindi then
            if name = "original" then return { rip: rip, dub: false }
            if fallback = invalid then fallback = { rip: rip, dub: Instr(1, name, "dub") > 0 }
        end if
    end for
    return fallback
end function

function DRC_EpisodeNumber(fTitle as String) as Integer
    parsed = SRC_ParseEpisode(fTitle)
    if parsed <> invalid then return parsed.ep
    found = SRC_Match("(?:episode|ep\.?)\s*[-._]?\s*(\d{1,3})", fTitle)
    if found <> "" then return found.ToInt()
    found = SRC_Match("(\d{1,3})\s*$", DRC_CleanQuality(fTitle))
    if found <> "" then return found.ToInt()
    return 0
end function

function DRC_CleanQuality(text as String) as String
    re = CreateObject("roRegex", "\s+(\d{3,4}p|DUB|hi DUB|ENG Subbed Full)\s*$", "i")
    return re.ReplaceAll(text.Trim(), "")
end function

' Returns { url, file, ext } for one file, or invalid.
function DRC_File(fid as String, disk as String) as Dynamic
    json = DRC_Get("interface=getFile&fid=" + fid + "&findex=" + disk)
    if json = invalid or json.hostInfo = invalid or json.fileInfo = invalid or json.fileInfo.Count() = 0 then return invalid
    info = json.fileInfo[0]
    path = MB_Str(info.url)
    host = MB_Str(json.hostInfo.host)
    if path = "" or host = "" then return invalid
    while Right(host, 1) = "/"
        host = Left(host, Len(host) - 1)
    end while
    while Left(path, 1) = "/"
        path = Mid(path, 2)
    end while
    file = MB_Str(info.filename)
    if file = "" then file = MB_Str(info.f_title)
    return { url: SRC_EncodeUrl("https://" + host + "/cdn/" + path), file: file, ext: LCase(MB_Str(info.ext)) }
end function

' Dramachi releases for a title (possibly empty), already checked and ranked.
function DRC_Search(title as String, year as String, stype as Integer, se as Integer, ep as Integer) as Object
    releases = []
    found = DRC_FindTitle(title, year, stype)
    if found = invalid then return releases
    details = DRC_Get("interface=title_v2&id=" + found.id)
    if details = invalid then return releases
    pick = DRC_PickRip(details, stype, se)
    if pick = invalid then return releases
    escaper = CreateObject("roUrlTransfer")
    list = DRC_Get("interface=eplist&season=" + escaper.Escape(pick.rip) + "&id=" + found.id)
    if list = invalid or list.episode_list = invalid then return releases

    files = []
    if stype = 2 then
        for each item in list.episode_list
            if DRC_EpisodeNumber(MB_Str(item.f_title)) = ep then files.Push(item)
        end for
    else
        files = list.episode_list
    end if

    caps = SRC_DeviceCaps()
    english = LCase(found.category) = "hollywood" or LCase(found.category) = "tv series" or pick.dub
    for each item in files
        if releases.Count() >= 3 then exit for
        fid = MB_Str(item.fid)
        disk = MB_Str(item.disk)
        target = invalid
        if fid <> "" and disk <> "" then target = DRC_File(fid, disk)
        codec = ""
        if target <> invalid then codec = SRC_SniffCodec(target.url)
        if codec <> "" then
            quality = MB_Str(item.quality)
            size = MB_Str(item.size)
            rel = {
                provider: "Dramachi"
                file: target.file
                size: size
                sizeGb: SRC_SizeGb(size)
                height: SRC_Height(quality + " " + target.file)
                codec: codec
                langs: { spa: false, eng: english, hin: false, other: not english }
                mirrors: []
                url: target.url
                ext: target.ext
            }
            rel.score = SRC_Score(rel, caps)
            rel.label = SRC_ReleaseLabel(rel)
            if rel.score > -1000 then releases.Push(rel)
        end if
    end for
    MB_Log("drc releases: " + releases.Count().ToStr())
    return releases
end function
