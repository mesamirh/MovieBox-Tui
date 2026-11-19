sub init()
    m.top.functionName = "runRequest"
end sub

sub runRequest()
    req = m.top.request
    if req = invalid then req = {}
    action = MB_Str(req.action)
    result = invalid
    if action = "home" then
        result = loadHome(MB_Str(req.tabId))
    else if action = "search" then
        result = loadSearch(MB_Str(req.query), MB_Int(req.page))
    else if action = "details" then
        result = loadDetails(MB_Str(req.id))
    else if action = "streams" then
        result = loadStreams(MB_Str(req.id), MB_Int(req.se), MB_Int(req.ep))
        if result.ok = true then addOnlineSpanish(result, req)
    else if action = "subsearch" then
        result = SUB_Search(MB_Str(req.title), MB_Str(req.year), MB_Int(req.stype), MB_Int(req.se), MB_Int(req.ep))
    else if action = "subfetch" then
        path = SUB_Download(MB_Str(req.url), MB_Str(req.id))
        if path <> "" then
            result = { ok: true, path: path }
        else
            result = { ok: false, error: "Couldn't download that subtitle. Try another one." }
        end if
    else if action = "altsearch" then
        result = SRC_Search(MB_Str(req.title), MB_Str(req.year), MB_Int(req.stype), MB_Int(req.se), MB_Int(req.ep))
    else if action = "altauto" then
        result = SRC_Auto(MB_Str(req.title), MB_Str(req.year), MB_Int(req.stype), MB_Int(req.se), MB_Int(req.ep))
        if result.ok = true and req.withSubs = true then
            result.captions = []
            addOnlineSpanish(result, req)
        end if
    else if action = "altresolve" then
        src = invalid
        if req.release <> invalid then src = SRC_Resolve(req.release)
        if src <> invalid then
            result = { ok: true, source: src }
            if req.withSubs = true then
                result.captions = []
                addOnlineSpanish(result, req)
            end if
        else
            result = { ok: false, error: "That file is not reachable right now. Try another one." }
        end if
    else if action = "addonmanifest" then
        result = ADD_FetchManifest(MB_Str(req.url))
    else if action = "captions" then
        result = { ok: true, captions: loadCaptions(MB_Str(req.id), MB_Str(req.resourceId)) }
    else
        result = { ok: false, error: "Unknown action" }
    end if
    result.action = action
    result.token = req.token
    MB_Log("task done: " + action)
    m.top.response = result
end sub

' ---------------------------------------------------------------------------
' Home / browse tabs
' ---------------------------------------------------------------------------

function loadHome(tabId as String) as Object
    ' Catalogue pages tolerate a short staleness window; 30 minutes keeps a tab
    ' switch instant without showing a yesterday's home page.
    res = MB_GetCached("/wefeed-mobile-bff/tab-operating?page=1&tabId=" + tabId + "&version=", 1800)
    if not res.ok then return { ok: false, error: res.error }

    groups = invalid
    if Type(res.data) = "roAssociativeArray" then groups = res.data.items
    if groups = invalid then return { ok: false, error: "MovieBox returned an empty page" }

    root = CreateObject("roSGNode", "ContentNode")
    seen = {}
    rowCount = 0
    for each group in groups
        if rowCount >= 18 then exit for
        gtype = MB_Str(group.type)
        title = MB_Html(MB_Str(group.title)).Trim()
        subjectsAllowed = not MB_IsHiddenRow(title)
        subjects = []
        kind = "poster"
        if gtype = "BANNER" then
            kind = "wide"
            title = "Featured"
            if group.banner <> invalid and group.banner.banners <> invalid then
                for each b in group.banner.banners
                    if b.subject <> invalid then
                        wideUrl = ""
                        if b.image <> invalid then wideUrl = MB_Str(b.image.url)
                        subjects.Push({ subject: b.subject, wide: wideUrl })
                    end if
                end for
            end if
        else if gtype = "RANKING_LIST_MULTI_TAB" then
            if group.rankingListData <> invalid and group.rankingListData.items <> invalid then
                tabs = group.rankingListData.items
                if tabs.Count() > 0 and tabs[0].subjects <> invalid then
                    for each s in tabs[0].subjects
                        subjects.Push({ subject: s, wide: "" })
                    end for
                end if
            end if
        else if gtype = "SUBJECTS_MOVIE" or gtype = "APPOINTMENT_LIST" then
            if group.subjects <> invalid then
                for each s in group.subjects
                    subjects.Push({ subject: s, wide: "" })
                end for
            end if
        else if gtype = "CUSTOM" then
            if group.customData <> invalid and group.customData.items <> invalid then
                for each c in group.customData.items
                    if c.subject <> invalid then subjects.Push({ subject: c.subject, wide: "" })
                end for
            end if
        end if

        items = []
        if not subjectsAllowed then subjects = []
        for each entry in subjects
            item = MB_NormalizeSubject(entry.subject)
            if item <> invalid then
                if kind = "wide" then
                    if entry.wide <> "" then item.wide = MB_Image(entry.wide, 640) else item.wide = item.backdrop
                    if entry.wide <> "" then item.backdrop = MB_Image(entry.wide, 1280)
                end if
                rowKey = kind + item.id
                titleKey = title + "|" + LCase(item.title) + "|" + item.year
                if seen[rowKey] = invalid and seen[titleKey] = invalid then
                    seen[rowKey] = true
                    seen[titleKey] = true
                    items.Push(item)
                end if
            end if
            if items.Count() >= 30 then exit for
        end for

        if items.Count() >= 3 then
            row = root.createChild("ContentNode")
            row.title = cleanRowTitle(title)
            row.addFields({ kind: kind })
            for each item in items
                row.appendChild(MB_ItemNode(item))
            end for
            rowCount = rowCount + 1
        end if
    end for

    MB_Log("home rows built: " + rowCount.ToStr())
    if rowCount = 0 then return { ok: false, error: "Nothing to show right now" }
    m.top.content = root
    return { ok: true, rows: rowCount }
end function

' Removes leading emoji/symbols the Roku fonts cannot render.
function cleanRowTitle(title as String) as String
    out = ""
    for i = 1 to Len(title)
        ch = Mid(title, i, 1)
        code = Asc(ch)
        if code < 8192 or (code >= 8208 and code <= 8231) then out = out + ch
    end for
    out = out.Trim()
    if out = "" then out = "Popular"
    return out
end function

' ---------------------------------------------------------------------------
' Search
' ---------------------------------------------------------------------------

function loadSearch(query as String, page as Integer) as Object
    if page < 1 then page = 1
    q = Chr(34)
    body = "{" + q + "keyword" + q + ":" + FormatJson(query) + "," + q + "page" + q + ":" + page.ToStr() + "," + q + "perPage" + q + ":20," + q + "subjectType" + q + ":0}"
    res = MB_PostCached("/wefeed-mobile-bff/subject-api/search/v2", body, 600)
    if not res.ok then return { ok: false, error: res.error }

    root = CreateObject("roSGNode", "ContentNode")
    count = 0
    hasMore = false
    seenTitles = {}
    if Type(res.data) = "roAssociativeArray" then
        if res.data.pager <> invalid and res.data.pager.hasMore = true then hasMore = true
        results = res.data.results
        if results <> invalid then
            for each topic in results
                if topic.subjects <> invalid then
                    for each s in topic.subjects
                        item = MB_NormalizeSubject(s)
                        if item <> invalid then
                            key = LCase(item.title) + "|" + item.year + "|" + item.stype.ToStr()
                            if seenTitles[key] = invalid then
                                seenTitles[key] = true
                                root.appendChild(MB_ItemNode(item))
                                count = count + 1
                            end if
                        end if
                    end for
                end if
            end for
        end if
    end if
    m.top.content = root
    return { ok: true, count: count, query: query, page: page, hasMore: hasMore }
end function

' ---------------------------------------------------------------------------
' Details
' ---------------------------------------------------------------------------

function loadDetails(id as String) as Object
    return loadDetailsInner(id, true)
end function

function loadDetailsInner(id as String, allowRedirect as Boolean) as Object
    res = MB_GetCached("/wefeed-mobile-bff/subject-api/get?subjectId=" + id, 86400)
    if not res.ok then return { ok: false, error: res.error }
    data = res.data
    if Type(data) <> "roAssociativeArray" then return { ok: false, error: "Title not found" }
    if data.subject <> invalid then data = data.subject

    item = MB_NormalizeSubject(data)
    if item = invalid then
        ' Detail pages can omit hasResource/subjectType on some titles; build leniently.
        names = MB_SplitTitle(MB_Str(data.title))
        item = {
            id: id, stype: MB_Int(data.subjectType), title: names.title, tag: names.tag
            year: MB_Year(data.releaseDate), genre: MB_Str(data.genre), rating: MB_Str(data.imdbRatingValue)
            duration: MB_Str(data.duration), country: MB_Str(data.countryName)
            description: MB_Html(MB_Str(data.description))
            poster: MB_Image(MB_CoverUrl(data), 320), backdrop: MB_Image(MB_StillUrl(data), 1280), wide: ""
        }
        if item.stype <> 2 then item.stype = 1
        if item.backdrop = "" then item.backdrop = MB_Image(MB_CoverUrl(data), 1280)
    end if
    item.posterLarge = MB_Image(MB_CoverUrl(data), 480)

    cast = []
    if data.staffList <> invalid then
        for each staff in data.staffList
            if cast.Count() < 6 and MB_Int(staff.staffType) = 1 then cast.Push(MB_Str(staff.name))
        end for
    end if
    item.cast = cast.Join(", ")

    dubs = []
    originalId = ""
    if data.dubs <> invalid then
        for each dub in data.dubs
            dubId = MB_Str(dub.subjectId)
            dubName = MB_DubName(MB_Str(dub.lanName), MB_Str(dub.lanCode))
            if dub.original = true then originalId = dubId
            if dubId <> "" and not MB_IsHiddenLanguage(dubName) then dubs.Push({ id: dubId, name: dubName, current: (dubId = id) })
        end for
    end if
    ' Opened a Hindi version (deep link, old history): show the original instead.
    if allowRedirect and MB_IsHiddenLanguage(MB_Str(data.title)) and originalId <> "" and originalId <> id then
        return loadDetailsInner(originalId, false)
    end if
    item.dubs = dubs

    seasons = []
    if item.stype = 2 then
        sres = MB_GetCached("/wefeed-mobile-bff/subject-api/season-info?subjectId=" + id, 86400)
        if sres.ok and Type(sres.data) = "roAssociativeArray" and sres.data.seasons <> invalid then
            for each s in sres.data.seasons
                se = MB_Int(s.se)
                maxEp = MB_Int(s.maxEp)
                if se > 0 and maxEp > 0 then seasons.Push({ se: se, maxEp: maxEp })
            end for
        end if
        if seasons.Count() = 0 then seasons.Push({ se: 1, maxEp: 1 })
    end if
    item.seasons = seasons
    return { ok: true, item: item }
end function

' ---------------------------------------------------------------------------
' Streams
' ---------------------------------------------------------------------------

function isNoticeUrl(url as String) as Boolean
    lower = LCase(url)
    if Instr(1, lower, "1c7de0bd3393702d9191801f15f88f8d") > 0 then return true
    if Instr(1, lower, "9a0461bc39da389663bf3dbb17091d3f") > 0 then return true
    if Instr(1, lower, "b164fbfb4347792950bdfbfb563d39d9") > 0 then return true
    if Instr(1, lower, "/notice.mp4") > 0 then return true
    if Instr(1, lower, "macdn.aoneroom.com") > 0 and Instr(1, lower, "/other/") > 0 then return true
    return false
end function

function codecLabel(codec as String) as String
    c = LCase(codec)
    if c = "hevc" or c = "h265" or c = "x265" then return "HEVC"
    if c = "h264" or c = "avc" or c = "x264" then return "H.264"
    if c = "" then return ""
    return UCase(codec)
end function

function formatSize(sizeText as String) as String
    if sizeText = "" then return ""
    bytes = Val(sizeText)
    if bytes <= 0 then return ""
    gb = bytes / 1073741824.0
    if gb >= 1 then return (Int(gb * 10 + 0.5) / 10).ToStr() + " GB"
    mb = bytes / 1048576.0
    return Int(mb + 0.5).ToStr() + " MB"
end function

function urlBase(url as String) as String
    q = Instr(1, url, "?")
    if q > 0 then return Left(url, q - 1)
    return url
end function

sub addMp4Source(sources as Object, seen as Object, entry as Object, se as Integer, ep as Integer)
    url = MB_Str(entry.resourceLink)
    if url = "" then url = MB_Str(entry.url)
    if url = "" or Left(url, 4) <> "http" or isNoticeUrl(url) then return
    if MB_Int(entry.se) <> se or MB_Int(entry.ep) <> ep then return
    base = urlBase(url)
    if seen[base] <> invalid then return
    seen[base] = true
    res = MB_Int(entry.resolution)
    codec = MB_Str(entry.codecName)
    sources.Push({
        kind: "mp4"
        url: url
        resolution: res
        codec: LCase(codec)
        size: formatSize(MB_Str(entry.size))
        resourceId: MB_Str(entry.resourceId)
        cookie: ""
        label: resLabel(res) + "  ·  " + codecLabel(codec) + "  ·  MP4"
    })
end sub

function resLabel(res as Integer) as String
    if res >= 2160 then return "4K"
    if res <= 0 then return "Auto"
    return res.ToStr() + "p"
end function

function loadStreams(id as String, se as Integer, ep as Integer) as Object
    sources = []
    seen = {}
    lastError = ""

    path = "/wefeed-mobile-bff/subject-api/resource/v2?subjectId=" + id + "&page=1&perPage=20&all=0&startPosition=1&endPosition=1&pagerMode=2&resolution=0&se=" + se.ToStr() + "&epFrom=" + ep.ToStr() + "&epTo=" + ep.ToStr() + "&isVip=true"
    res = MB_Get(path)
    maxMp4Duration = 0
    if res.ok and Type(res.data) = "roAssociativeArray" and res.data.list <> invalid then
        for each entry in res.data.list
            addMp4Source(sources, seen, entry, se, ep)
            d = MB_Int(entry.duration)
            if MB_Int(entry.se) = se and MB_Int(entry.ep) = ep and d > maxMp4Duration then maxMp4Duration = d
        end for
    else if not res.ok then
        lastError = MB_Str(res.error)
    end if

    playPath = "/wefeed-mobile-bff/subject-api/play-info/v2?subjectId=" + id
    if se <> 0 or ep <> 0 then playPath = playPath + "&se=" + se.ToStr() + "&ep=" + ep.ToStr()
    pres = MB_Get(playPath)
    if pres.ok and Type(pres.data) = "roAssociativeArray" and pres.data.streams <> invalid then
        for each stream in pres.data.streams
            url = MB_Str(stream.url)
            isPreview = Instr(1, url, "/preview/") > 0
            streamDur = MB_Int(stream.duration)
            ' Short "preview" cuts are served to some sessions; skip them when a full file exists.
            if maxMp4Duration > 0 and streamDur > 0 and streamDur < maxMp4Duration * 0.7 then isPreview = true
            MB_Log("play-info stream " + url + " preview=" + isPreview.ToStr() + " dur=" + streamDur.ToStr())
            if url <> "" and Left(url, 4) = "http" and not isNoticeUrl(url) and not isPreview then
                best = 0
                for each r in MB_Str(stream.resolutions).Split(",")
                    v = r.Trim().ToInt()
                    if v > best then best = v
                end for
                codec = MB_Str(stream.codecName)
                fmt = UCase(MB_Str(stream.format))
                kind = "dash"
                if fmt = "HLS" or Instr(1, url, ".m3u8") > 0 then kind = "hls"
                if fmt = "MP4" or Instr(1, LCase(urlBase(url)), ".mp4") > 0 then kind = "mp4"
                cookie = MB_Str(stream.signCookie).Trim()
                if Right(cookie, 1) = ";" then cookie = Left(cookie, Len(cookie) - 1)
                if seen[urlBase(url)] = invalid then
                    seen[urlBase(url)] = true
                    sources.Push({
                        kind: kind
                        url: url
                        resolution: best
                        codec: LCase(codec)
                        size: formatSize(MB_Str(stream.size))
                        resourceId: MB_Str(stream.id)
                        cookie: cookie
                        label: resLabel(best) + "  ·  " + codecLabel(codec) + "  ·  Adaptive"
                    })
                end if
            end if
        end for
    end if

    ' Fallback: download detectors attached to the subject.
    if sources.Count() = 0 then
        dres = MB_Get("/wefeed-mobile-bff/subject-api/get?subjectId=" + id)
        if dres.ok and Type(dres.data) = "roAssociativeArray" and dres.data.resourceDetectors <> invalid then
            for each detector in dres.data.resourceDetectors
                if detector.resolutionList <> invalid then
                    for each entry in detector.resolutionList
                        addMp4Source(sources, seen, entry, se, ep)
                    end for
                end if
            end for
        end if
    end if

    if sources.Count() = 0 then
        msg = "No playable sources were found for this title."
        if lastError <> "" then msg = msg + " (" + lastError + ")"
        return { ok: false, error: msg }
    end if

    ' Keep only codecs this Roku can decode (fall back to everything if none match).
    info = CreateObject("roDeviceInfo")
    hevcOk = info.CanDecodeVideo({ Codec: "hevc" }).result = true
    playable = []
    for each s in sources
        isHevc = (s.codec = "hevc" or s.codec = "h265")
        if hevcOk or not isHevc then playable.Push(s)
    end for
    if playable.Count() = 0 then playable = sources

    ' Highest resolution first; progressive MP4 before adaptive at equal height.
    for i = 1 to playable.Count() - 1
        current = playable[i]
        j = i - 1
        while j >= 0 and sourceBetter(current, playable[j])
            playable[j + 1] = playable[j]
            j = j - 1
        end while
        playable[j + 1] = current
    end for

    captions = []
    for each s in playable
        if s.resourceId <> "" then
            captions = loadCaptions(id, s.resourceId)
            if captions.Count() > 0 then exit for
        end if
    end for

    return { ok: true, sources: playable, captions: captions, se: se, ep: ep }
end function

' Progressive MP4 needs no cookies and seeks reliably, so it gets a head start
' over adaptive streams (a 720p MP4 beats 1080p DASH; 1080p DASH beats 480p MP4).
function sourceScore(s as Object) as Integer
    score = s.resolution
    if s.kind = "mp4" then score = score + 400
    return score
end function

function sourceBetter(a as Object, b as Object) as Boolean
    return sourceScore(a) > sourceScore(b)
end function

function loadCaptions(id as String, resourceId as String) as Object
    out = []
    if resourceId = "" then return out
    res = MB_Get("/wefeed-mobile-bff/subject-api/get-ext-captions?subjectId=" + id + "&resourceId=" + resourceId)
    if not res.ok or Type(res.data) <> "roAssociativeArray" or res.data.extCaptions = invalid then return out
    seen = {}
    for each cap in res.data.extCaptions
        url = MB_Str(cap.url)
        size = MB_Int(cap.size)
        name = MB_Str(cap.lanName)
        if name = "" then name = MB_Str(cap.lan)
        if name = "" then name = "Unknown"
        lan = LCase(MB_Str(cap.lan))
        skip = (url = "" or Instr(1, url, "aa348f2541d13ffe") > 0 or (size > 0 and size <= 50))
        if lan = "in" and size <= 100 then skip = true
        key = SubLangKey(lan)
        ' Hindi is hidden channel-wide; other names use Latin script so the font can draw them.
        if key = "hin" then skip = true
        if not skip and seen[url] = invalid then
            seen[url] = true
            out.Push({ name: SubLangName(key), lang: key, url: url })
        end if
    end for
    ' Spanish first, then English, then the rest in API order.
    ordered = []
    for each pass in [0, 1, 2]
        for each cap in out
            if SUB_Priority(cap.lang) = pass then ordered.Push(cap)
        end for
    end for
    return ordered
end function

' Built-in captions rarely include Spanish; fetch the best online one up front so
' it is ready before playback starts (never blocks playback if the lookup fails).
sub addOnlineSpanish(result as Object, req as Object)
    for each cap in result.captions
        if SubLangKey(MB_Str(cap.lang)) = "spa" then return
    end for
    title = MB_Str(req.title)
    if title = "" then return
    online = SUB_AutoSpanish(title, MB_Str(req.year), MB_Int(req.stype), MB_Int(req.se), MB_Int(req.ep))
    if online <> invalid then result.captions.Unshift(online)
end sub
