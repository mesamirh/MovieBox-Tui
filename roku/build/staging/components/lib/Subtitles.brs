' Online subtitle lookup (Task threads only).
'
' 1. Cinemeta (Stremio) resolves the title + year to an IMDb id.
' 2. The public OpenSubtitles v3 Stremio addon lists subtitles for that id
'    (or id:season:episode), already converted to UTF-8 SRT.
' 3. The chosen file is saved under tmp:/subs/ with a .srt name so the Roku
'    player recognises the format.

function SUB_UserAgent() as String
    return "MovieBox-Roku/1.0"
end function

function SUB_GetJson(url as String) as Dynamic
    ' Short timeout: the automatic lookup runs before playback and must not stall it.
    res = MB_HttpRequest("GET", url, invalid, [["User-Agent", SUB_UserAgent()], ["Accept", "application/json"]], 6000)
    if res.code < 200 or res.code >= 300 then
        MB_Log("subs GET " + url + " -> " + res.code.ToStr())
        return invalid
    end if
    return ParseJson(res.body)
end function

' Lowercase letters and digits only, for fuzzy title comparison.
function SUB_Normalize(text as String) as String
    out = ""
    lower = LCase(text)
    for i = 1 to Len(lower)
        ch = Mid(lower, i, 1)
        code = Asc(ch)
        if (code >= 97 and code <= 122) or (code >= 48 and code <= 57) then out = out + ch
    end for
    return out
end function

function SUB_FindImdbId(title as String, year as String, kind as String) as String
    escaper = CreateObject("roUrlTransfer")
    json = SUB_GetJson("https://v3-cinemeta.strem.io/catalog/" + kind + "/top/search=" + escaper.Escape(title) + ".json")
    if json = invalid or json.metas = invalid then return ""

    wanted = SUB_Normalize(title)
    bestId = ""
    bestScore = -1
    corroborated = false
    index = 0
    for each meta in json.metas
        imdb = MB_Str(meta.imdb_id)
        if imdb = "" then imdb = MB_Str(meta.id)
        if Left(imdb, 2) = "tt" then
            score = 0
            if SUB_Normalize(MB_Str(meta.name)) = wanted then score = score + 4
            metaYear = Left(MB_Str(meta.releaseInfo), 4)
            if year <> "" and metaYear <> "" then
                diff = Abs(metaYear.ToInt() - year.ToInt())
                if diff = 0 then
                    score = score + 3
                else if diff = 1 then
                    score = score + 1
                end if
            end if
            ' Earlier results are more relevant; use the position as a tiebreaker.
            if score > 0 then corroborated = true
            score = score * 10 - index
            if score > bestScore then
                bestScore = score
                bestId = imdb
            end if
        end if
        index = index + 1
        if index >= 8 then exit for
    end for
    ' Never guess: a wrong IMDb id pulls subtitles for a different film, which the
    ' viewer sees as broken subtitles. Mirror the TUI, which demands a corroborated
    ' title before trusting a lookup.
    if not corroborated then
        MB_Log("subs: no corroborated imdb match for " + title)
        return ""
    end if
    return bestId
end function

function SUB_Priority(langKey as String) as Integer
    if langKey = "spa" then return 0
    if langKey = "eng" then return 1
    return 2
end function

' Returns { ok, imdb, subs: [{ id, lang, name, label, url }] } with Spanish first.
function SUB_Search(title as String, year as String, stype as Integer, se as Integer, ep as Integer) as Object
    kind = "movie"
    if stype = 2 then kind = "series"
    imdb = SUB_FindImdbId(title, year, kind)
    if imdb = "" then return { ok: false, error: "Couldn't identify this title online." }

    path = imdb
    if kind = "series" then path = imdb + ":" + se.ToStr() + ":" + ep.ToStr()
    json = SUB_GetJson("https://opensubtitles-v3.strem.io/subtitles/" + kind + "/" + path + ".json")
    if json = invalid or json.subtitles = invalid then return { ok: false, error: "The subtitle service is not responding." }

    buckets = [[], [], []]
    seen = {}
    for each s in json.subtitles
        url = MB_Str(s.url)
        id = MB_Str(s.id)
        if url <> "" and id <> "" and seen[id] = invalid then
            seen[id] = true
            key = SubLangKey(MB_Str(s.lang))
            release = MB_Str(s.movieReleaseName)
            if release = "" then release = MB_Str(s.subtitleFileName)
            if Len(release) > 52 then release = Left(release, 50) + "..."
            name = SubLangName(key)
            buckets[SUB_Priority(key)].Push({ id: id, lang: key, name: name, label: name + "   ·   " + release, url: url })
        end if
    end for

    subs = []
    for each bucket in buckets
        for each item in bucket
            if subs.Count() < 60 then subs.Push(item)
        end for
    end for
    if subs.Count() = 0 then return { ok: false, error: "No subtitles found online for this title." }
    MB_Log("subs " + imdb + ": " + subs.Count().ToStr() + " found, first " + subs[0].lang)
    return { ok: true, imdb: imdb, subs: subs }
end function

' Downloads one subtitle to tmp:/subs/<id>.srt; returns the path or "" on failure.
function SUB_Download(url as String, id as String) as String
    fs = CreateObject("roFileSystem")
    if not fs.Exists("tmp:/subs") then fs.CreateDirectory("tmp:/subs")
    safeId = SUB_Normalize(id)
    if safeId = "" then safeId = "sub"
    path = "tmp:/subs/" + safeId + ".srt"
    if fs.Exists(path) then return path

    port = CreateObject("roMessagePort")
    xfer = CreateObject("roUrlTransfer")
    xfer.SetMessagePort(port)
    xfer.SetUrl(url)
    xfer.SetCertificatesFile("common:/certs/ca-bundle.crt")
    xfer.InitClientCertificates()
    xfer.EnableEncodings(true)
    xfer.AddHeader("User-Agent", SUB_UserAgent())
    if not xfer.AsyncGetToFile(path) then return ""
    msg = Wait(8000, port)
    if Type(msg) <> "roUrlEvent" then
        xfer.AsyncCancel()
        fs.Delete(path)
        return ""
    end if
    if msg.GetResponseCode() <> 200 then
        fs.Delete(path)
        return ""
    end if
    ' Reject error pages and empty files: a real SRT has timing arrows.
    text = ReadAsciiFile(path)
    if Len(text) < 40 or Instr(1, text, "-->") = 0 then
        fs.Delete(path)
        return ""
    end if
    return path
end function

' Picks the best Spanish subtitle and downloads it; returns a caption entry or invalid.
function SUB_AutoSpanish(title as String, year as String, stype as Integer, se as Integer, ep as Integer) as Dynamic
    found = SUB_Search(title, year, stype, se, ep)
    if found.ok <> true then return invalid
    tries = 0
    for each s in found.subs
        if s.lang <> "spa" then exit for
        path = SUB_Download(s.url, s.id)
        if path <> "" then return { name: s.name + " (online)", lang: "spa", url: path, online: true }
        tries = tries + 1
        if tries >= 2 then exit for
    end for
    return invalid
end function
