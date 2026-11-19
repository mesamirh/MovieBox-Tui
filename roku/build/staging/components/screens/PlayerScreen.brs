sub init()
    m.video = m.top.findNode("video")
    m.loading = m.top.findNode("loading")
    m.loadingArt = m.top.findNode("loadingArt")
    m.loadingTitle = m.top.findNode("loadingTitle")
    m.loadingSub = m.top.findNode("loadingSub")
    m.spinner = m.top.findNode("spinner")
    m.dialog = m.top.findNode("dialog")
    m.toast = m.top.findNode("toast")
    m.toastBg = m.top.findNode("toastBg")
    m.toastText = m.top.findNode("toastText")
    m.toastTimer = m.top.findNode("toastTimer")

    m.video.notificationInterval = 1
    m.video.observeField("state", "onVideoState")
    m.video.observeField("position", "onVideoPosition")
    m.video.observeField("availableAudioTracks", "onAudioTracks")
    m.dialog.observeField("selected", "onDialogSelected")
    m.dialog.observeField("dismissed", "onDialogDismissed")
    m.toastTimer.observeField("fire", "hideToast")

    m.sources = []
    m.captions = []
    m.onlineSubs = []
    m.forcedSub = ""
    m.sourceIndex = 0
    m.lastSaved = 0
    m.closing = false
    m.started = false
    m.altReleases = []
    m.altTried = false
    m.lastError = ""
    m.optionIds = []
end sub

sub onStart()
    item = m.top.item
    if item = invalid then return
    m.se = m.top.se
    m.ep = m.top.ep
    m.resumeAt = m.top.resume
    m.loadingArt.uri = item.backdrop
    if m.top.altMode = true then
        m.altTried = true
        showLoading("Searching other providers...")
        startAltTask({ action: "altsearch" }, "onAltSearch")
    else
        fetchStreams()
    end if
end sub

function episodeLabel() as String
    if m.top.item.stype = 2 and m.ep > 0 then return EpisodeCode(m.se, m.ep)
    return ""
end function

sub showLoading(sub1 as String)
    m.loading.visible = true
    title = m.top.item.title
    code = episodeLabel()
    if code <> "" then title = title + "   " + code
    m.loadingTitle.text = title
    m.loadingSub.text = sub1
    m.spinner.running = true
end sub

sub hideLoading()
    m.loading.visible = false
    m.spinner.running = false
end sub

sub fetchStreams()
    m.started = false
    m.video.control = "stop"
    showLoading("Finding the best stream and subtitles...")
    m.task = CreateObject("roSGNode", "ApiTask")
    m.task.observeField("response", "onStreams")
    item = m.top.item
    m.task.request = { action: "streams", id: item.sid, se: m.se, ep: m.ep, title: item.title, year: item.year, stype: item.stype }
    m.task.control = "run"
end sub

sub onStreams(event as Object)
    res = event.getRoSGNode().response
    m.task = invalid
    if m.closing then return
    if res.ok <> true then
        autoAlternatives(res.error)
        return
    end if
    m.sources = res.sources
    m.captions = res.captions
    #if DEBUG
        for each c in m.captions
            print "[player] caption "; c.name; " "; c.lang; " "; c.url
        end for
    #end if
    m.sourceIndex = pickSourceIndex()
    playSource(m.sourceIndex, m.resumeAt)
end sub

' Honours the saved quality cap (e.g. 720) while preferring the best match below it.
function pickSourceIndex() as Integer
    cap = PrefGet("maxRes", "0").ToInt()
    if cap <= 0 then return 0
    for i = 0 to m.sources.Count() - 1
        if m.sources[i].resolution <= cap then return i
    end for
    return m.sources.Count() - 1
end function

function langCode(code as String) as String
    map = {
        en: "eng", es: "spa", fr: "fre", de: "ger", it: "ita", pt: "por", ru: "rus", ar: "ara",
        hi: "hin", ta: "tam", te: "tel", ml: "mal", kn: "kan", bn: "ben", ur: "urd", zh: "chi",
        ja: "jpn", ko: "kor", tr: "tur", id: "ind", in: "ind", ms: "may", th: "tha", vi: "vie",
        fa: "per", pl: "pol", nl: "dut", sv: "swe", ro: "rum", el: "gre", he: "heb", fil: "fil"
    }
    c = LCase(code)
    if map[c] <> invalid then return map[c]
    if Len(c) = 3 then return c
    return "und"
end function

sub playSource(index as Integer, startAt as Integer)
    if index < 0 or index >= m.sources.Count() then
        failAndExplain("None of the available streams could be played on this Roku.")
        return
    end if
    src = m.sources[index]
    m.sourceIndex = index
    showLoading("Starting " + src.label + "...")

    content = CreateObject("roSGNode", "ContentNode")
    content.url = src.url
    content.streamFormat = src.kind
    title = m.top.item.title
    code = episodeLabel()
    if code <> "" then title = title + "  •  " + code
    content.title = title
    content.SDPosterUrl = m.top.item.HDPosterUrl
    content.HDPosterUrl = m.top.item.HDPosterUrl
    if startAt > 5 then content.PlayStart = startAt

    ' One agent carries the signed CDN cookie for the manifest and every segment request.
    agent = CreateObject("roHttpAgent")
    agent.SetCertificatesFile("common:/certs/ca-bundle.crt")
    agent.InitClientCertificates()
    if src.userAgent <> invalid and src.userAgent <> "" then agent.AddHeader("User-Agent", src.userAgent)
    if src.headers <> invalid then
        ' Addon streams may require their own headers (Stremio proxyHeaders).
        for each h in src.headers
            agent.AddHeader(h[0], h[1])
        end for
    end if
    if src.cookie <> "" then
        agent.AddHeader("Cookie", src.cookie)
        agent.AddHeader("Referer", "https://sportslive.wine")
    end if
    m.video.setHttpAgent(agent)

    tracks = []
    for each cap in m.captions
        tracks.Push({ TrackName: cap.url, Language: langCode(cap.lang), Description: cap.name })
    end for
    if tracks.Count() > 0 then content.SubtitleTracks = tracks

    m.video.content = content
    applySubtitlePref()
    m.video.control = "play"
    m.video.setFocus(true)
end sub

' The preference stores a language key ("spa", "eng"...); a track picked by hand
' in this session (m.forcedSub) wins over the first track in that language.
sub applySubtitlePref()
    pref = PrefGet("subtitles", "off")
    if pref = "off" or m.captions.Count() = 0 then
        m.video.globalCaptionMode = "Off"
        return
    end if
    if m.forcedSub <> invalid and m.forcedSub <> "" then
        for each cap in m.captions
            if cap.url = m.forcedSub then
                m.video.subtitleTrack = cap.url
                m.video.globalCaptionMode = "On"
                return
            end if
        end for
    end if
    for each cap in m.captions
        if SubLangKey(cap.lang) = SubLangKey(pref) or LCase(cap.name) = LCase(pref) then
            m.video.subtitleTrack = cap.url
            m.video.globalCaptionMode = "On"
            return
        end if
    end for
    m.video.globalCaptionMode = "Off"
end sub

sub onVideoState()
    state = m.video.state
    #if DEBUG
        print "[player] state "; state; " "; m.video.errorMsg
    #end if
    if m.closing then return
    if state = "playing" then
        m.started = true
        hideLoading()
    else if state = "paused" then
        ' A deliberate pause is a good moment to commit progress to flash.
        saveProgress(false, true)
    else if state = "error" then
        failed = invalid
        if m.sourceIndex >= 0 and m.sourceIndex < m.sources.Count() then failed = m.sources[m.sourceIndex]
        position = m.video.position
        if position < 5 then position = m.resumeAt
        msg = m.video.errorMsg
        if msg = invalid or msg = "" then msg = "Playback error"
        failedLabel = "this stream"
        if failed <> invalid and failed.label <> invalid and failed.label <> "" then failedLabel = failed.label
        if failed <> invalid and failed.provider <> invalid then
            ' A file picked from another provider: let the user choose a different one.
            m.video.control = "stop"
            hideLoading()
            openAltList("That file couldn't be played on this Roku (" + msg + "). Pick another one.")
        else if m.sourceIndex + 1 < m.sources.Count() and m.sources[m.sourceIndex + 1].provider = invalid then
            showToast("Stream failed (" + failedLabel + "). Trying another source...")
            playSource(m.sourceIndex + 1, Int(position))
        else
            autoAlternatives(msg)
        end if
    else if state = "finished" then
        saveProgress(true, true)
        nextEpisode = findNextEpisode()
        if nextEpisode <> invalid then
            m.se = nextEpisode.se
            m.ep = nextEpisode.ep
            m.resumeAt = 0
            showToast("Up next: " + EpisodeCode(m.se, m.ep))
            fetchStreams()
        else
            close()
        end if
    end if
end sub

function findNextEpisode() as Dynamic
    if m.top.item.stype <> 2 then return invalid
    seasons = m.top.seasons
    if seasons = invalid then return invalid
    for i = 0 to seasons.Count() - 1
        s = seasons[i]
        if s.se = m.se then
            if m.ep < s.maxEp then return { se: m.se, ep: m.ep + 1 }
            if i + 1 < seasons.Count() then return { se: seasons[i + 1].se, ep: 1 }
            return invalid
        end if
    end for
    return invalid
end function

sub onVideoPosition()
    if not m.started then return
    position = Int(m.video.position)
    ' Periodic ticks stay in memory: the registry is flash-backed and flushing
    ' every 15 seconds is needless wear. Pause, finish and close flush.
    if Abs(position - m.lastSaved) >= 15 then saveProgress(false, false)
end sub

sub saveProgress(finished as Boolean, flush as Boolean)
    if not m.started then return
    position = Int(m.video.position)
    duration = Int(m.video.duration)
    if finished and duration > 0 then position = duration
    if position < 1 then return
    m.lastSaved = position
    HistorySave(SnapshotItem(m.top.item), m.se, m.ep, position, duration, flush)
end sub

sub failAndExplain(message as String)
    m.video.control = "stop"
    hideLoading()
    m.dialog.tag = "error"
    m.dialog.title = "Can't play this title"
    m.dialog.message = message
    m.dialog.currentIndex = -1
    m.dialog.options = ["Other providers", "Try MovieBox again", "Go back"]
    m.dialog.visible = true
    m.dialog.setFocus(true)
end sub

sub openOptions()
    subName = "Off"
    if m.video.globalCaptionMode = "On" then
        for each cap in m.captions
            if cap.url = m.video.subtitleTrack then subName = cap.name
        end for
    end if
    options = []
    m.optionIds = []
    if m.sources.Count() > 0 then
        options.Push("Quality:  " + m.sources[m.sourceIndex].label)
        m.optionIds.Push("quality")
        options.Push("Subtitles:  " + subName)
        m.optionIds.Push("subtitles")
    end if
    tracks = m.video.availableAudioTracks
    if tracks <> invalid and tracks.Count() > 1 then
        options.Push("Audio:  " + audioTrackName(currentAudio(tracks)))
        m.optionIds.Push("audio")
    end if
    options.Push("Other providers...")
    m.optionIds.Push("alt")
    options.Push("Close")
    m.optionIds.Push("close")
    m.dialog.tag = "options"
    m.dialog.title = "Playback Options"
    m.dialog.message = ""
    m.dialog.currentIndex = -1
    m.dialog.options = options
    m.dialog.visible = true
    m.dialog.setFocus(true)
end sub

sub openQuality()
    names = []
    for each s in m.sources
        label = s.label
        if s.size <> "" then label = label + "  ·  " + s.size
        names.Push(label)
    end for
    m.dialog.tag = "quality"
    m.dialog.title = "Quality"
    m.dialog.message = "Your choice is remembered for next time."
    m.dialog.currentIndex = m.sourceIndex
    m.dialog.options = names
    m.dialog.visible = true
    m.dialog.setFocus(true)
end sub

sub openSubtitles()
    names = ["Off"]
    current = 0
    for i = 0 to m.captions.Count() - 1
        names.Push(m.captions[i].name)
        if m.video.globalCaptionMode = "On" and m.captions[i].url = m.video.subtitleTrack then current = i + 1
    end for
    names.Push("Search online subtitles...")
    m.dialog.tag = "subtitles"
    m.dialog.title = "Subtitles"
    m.dialog.message = ""
    if m.captions.Count() = 0 then m.dialog.message = "This stream has no subtitles. Search online to find one (Spanish first)."

    m.dialog.currentIndex = current
    m.dialog.options = names
    m.dialog.visible = true
    m.dialog.setFocus(true)
end sub

sub closeDialog()
    m.dialog.visible = false
    m.video.setFocus(true)
end sub

sub onDialogDismissed()
    if m.dialog.tag = "error" then
        close()
        return
    end if
    if m.dialog.tag = "alt" and not isActive() then
        m.dialog.visible = false
        message = m.lastError
        if message = "" then message = "Pick a source to start playback."
        failAndExplain(message)
        return
    end if
    closeDialog()
end sub

sub onDialogSelected()
    tag = m.dialog.tag
    index = m.dialog.selected
    closeDialog()
    if tag = "error" then
        if index = 0 then
            showAlternatives()
        else if index = 1 then
            fetchStreams()
        else
            close()
        end if
    else if tag = "options" then
        id = ""
        if index >= 0 and index < m.optionIds.Count() then id = m.optionIds[index]
        if id = "quality" then
            openQuality()
        else if id = "subtitles" then
            openSubtitles()
        else if id = "audio" then
            openAudio()
        else if id = "alt" then
            showAlternatives()
        end if
    else if tag = "alt" then
        if index >= 0 and index < m.altReleases.Count() then resolveAlt(m.altReleases[index])
    else if tag = "audio" then
        tracks = m.video.availableAudioTracks
        if tracks <> invalid and index >= 0 and index < tracks.Count() then
            PrefSet("audioLang", SubLangKey(tracks[index].Language))
            m.video.audioTrack = tracks[index].Track
            showToast("Audio: " + audioTrackName(tracks[index]))
        end if
    else if tag = "quality" then
        if index <> m.sourceIndex then
            PrefSet("maxRes", m.sources[index].resolution.ToStr())
            position = Int(m.video.position)
            playSource(index, position)
        end if
    else if tag = "subtitles" then
        if index = 0 then
            PrefSet("subtitles", "off")
            m.forcedSub = ""
            m.video.globalCaptionMode = "Off"
        else if index > m.captions.Count() then
            searchOnlineSubtitles()
        else
            cap = m.captions[index - 1]
            PrefSet("subtitles", SubLangKey(cap.lang))
            m.forcedSub = cap.url
            m.video.subtitleTrack = cap.url
            m.video.globalCaptionMode = "On"
        end if
    else if tag = "online" then
        if index >= 0 and index < m.onlineSubs.Count() then downloadOnlineSubtitle(m.onlineSubs[index])
    end if
end sub

' ---------------------------------------------------------------------------
' Online subtitles (OpenSubtitles via Stremio), Spanish listed first
' ---------------------------------------------------------------------------

sub searchOnlineSubtitles()
    item = m.top.item
    showToast("Searching subtitles online...")
    cancelSubTask()
    m.subTask = CreateObject("roSGNode", "ApiTask")
    m.subTask.observeField("response", "onOnlineSearch")
    m.subTask.request = { action: "subsearch", title: item.title, year: item.year, stype: item.stype, se: m.se, ep: m.ep }
    m.subTask.control = "run"
end sub

sub onOnlineSearch(event as Object)
    res = event.getRoSGNode().response
    m.subTask = invalid
    if m.closing then return
    if res.ok <> true then
        showToast(res.error)
        return
    end if
    hideToast()
    m.onlineSubs = res.subs
    labels = []
    spanish = 0
    for each s in res.subs
        labels.Push(s.label)
        if s.lang = "spa" then spanish = spanish + 1
    end for
    message = spanish.ToStr() + " in Spanish, " + res.subs.Count().ToStr() + " in total. Pick one that matches your video release."
    if spanish = 0 then message = "No Spanish subtitles found; showing other languages."
    m.dialog.tag = "online"
    m.dialog.title = "Online Subtitles"
    m.dialog.message = message
    m.dialog.currentIndex = -1
    m.dialog.options = labels
    m.dialog.visible = true
    m.dialog.setFocus(true)
end sub

sub downloadOnlineSubtitle(choice as Object)
    m.pendingSub = choice
    showToast("Downloading subtitle...")
    cancelSubTask()
    m.subTask = CreateObject("roSGNode", "ApiTask")
    m.subTask.observeField("response", "onOnlineDownload")
    m.subTask.request = { action: "subfetch", url: choice.url, id: choice.id }
    m.subTask.control = "run"
end sub

sub onOnlineDownload(event as Object)
    res = event.getRoSGNode().response
    m.subTask = invalid
    if m.closing then return
    if res.ok <> true then
        showToast(res.error)
        return
    end if
    choice = m.pendingSub
    exists = false
    for each cap in m.captions
        if cap.url = res.path then exists = true
    end for
    if not exists then m.captions.Unshift({ name: choice.name + " (online)", lang: choice.lang, url: res.path, online: true })
    PrefSet("subtitles", choice.lang)
    m.forcedSub = res.path
    ' New tracks only register when the content is reloaded; resume at the same spot.
    position = Int(m.video.position)
    if position < 1 then position = m.resumeAt
    playSource(m.sourceIndex, position)
    showToast("Subtitles: " + choice.name)
end sub

' ---------------------------------------------------------------------------
' Other providers (4KHDHub and Dramachi), used when MovieBox can't play a title
' ---------------------------------------------------------------------------

sub startAltTask(req as Object, callback as String)
    item = m.top.item
    req.title = item.title
    req.year = item.year
    req.stype = item.stype
    req.se = m.se
    req.ep = m.ep
    req.withSubs = m.captions.Count() = 0
    ' Only one provider lookup may be in flight; a second one must replace it
    ' rather than race it.
    cancelAltTask()
    m.altTask = CreateObject("roSGNode", "ApiTask")
    m.altTask.observeField("response", callback)
    m.altTask.request = req
    m.altTask.control = "run"
end sub

sub cancelSubTask()
    if m.subTask = invalid then return
    m.subTask.unobserveField("response")
    m.subTask.control = "stop"
    m.subTask = invalid
end sub

sub cancelAltTask()
    if m.altTask = invalid then return
    m.altTask.unobserveField("response")
    m.altTask.control = "stop"
    m.altTask = invalid
end sub

' Whether something is on screen (playing, paused or loading a stream).
function isActive() as Boolean
    state = m.video.state
    return state = "playing" or state = "paused" or state = "buffering"
end function

function resumePoint() as Integer
    position = Int(m.video.position)
    if position < 5 then position = m.resumeAt
    return position
end function

' MovieBox failed: look for the title on other providers before giving up.
sub autoAlternatives(reason as String)
    m.lastError = reason
    if m.altTried then
        if m.altReleases.Count() > 0 then
            m.video.control = "stop"
            hideLoading()
            openAltList(reason + "  Pick a file from another provider.")
        else
            failAndExplain(reason)
        end if
        return
    end if
    m.altTried = true
    m.video.control = "stop"
    showLoading("MovieBox can't play this. Searching other providers...")
    startAltTask({ action: "altauto" }, "onAltAuto")
end sub

sub onAltAuto(event as Object)
    res = event.getRoSGNode().response
    m.altTask = invalid
    if m.closing then return
    if res.releases <> invalid then m.altReleases = res.releases
    if res.ok = true then
        if res.captions <> invalid and m.captions.Count() = 0 then m.captions = res.captions
        playAltSource(res.source)
    else
        failAndExplain(m.lastError + Chr(10) + res.error)
    end if
end sub

sub showAlternatives()
    if m.altReleases.Count() > 0 then
        openAltList("")
        return
    end if
    m.altTried = true
    if isActive() then
        showToast("Searching other providers...")
    else
        showLoading("Searching other providers...")
    end if
    startAltTask({ action: "altsearch" }, "onAltSearch")
end sub

sub onAltSearch(event as Object)
    res = event.getRoSGNode().response
    m.altTask = invalid
    if m.closing then return
    if res.ok = true then
        m.altReleases = res.releases
        hideLoadingIfIdle()
        openAltList("")
    else if isActive() then
        showToast(res.error)
    else
        if m.lastError = "" then m.lastError = res.error
        failAndExplain(res.error)
    end if
end sub

sub hideLoadingIfIdle()
    if not isActive() then hideLoading()
end sub

sub openAltList(message as String)
    names = []
    for each rel in m.altReleases
        names.Push(rel.label)
    end for
    if message = "" then message = "Files found on other sites, best match first. Bigger files need a faster connection."
    m.dialog.tag = "alt"
    m.dialog.title = "Other Providers"
    m.dialog.message = message
    m.dialog.currentIndex = -1
    m.dialog.options = names
    m.dialog.visible = true
    m.dialog.setFocus(true)
end sub

sub resolveAlt(rel as Object)
    if isActive() then
        showToast("Connecting to " + rel.provider + "...")
    else
        showLoading("Connecting to " + rel.provider + "...")
    end if
    startAltTask({ action: "altresolve", release: rel }, "onAltResolve")
end sub

sub onAltResolve(event as Object)
    res = event.getRoSGNode().response
    m.altTask = invalid
    if m.closing then return
    if res.ok = true then
        if res.captions <> invalid and m.captions.Count() = 0 then m.captions = res.captions
        playAltSource(res.source)
    else
        hideLoadingIfIdle()
        openAltList(res.error)
    end if
end sub

sub playAltSource(src as Object)
    index = -1
    for i = 0 to m.sources.Count() - 1
        if m.sources[i].url = src.url then index = i
    end for
    if index < 0 then
        m.sources.Push(src)
        index = m.sources.Count() - 1
    end if
    showToast("Playing from " + src.provider)
    playSource(index, resumePoint())
end sub

' ---------------------------------------------------------------------------
' Audio tracks (multi-language files from other providers)
' ---------------------------------------------------------------------------

' Prefers the saved language, then Spanish, then English, and never Hindi.
sub onAudioTracks()
    tracks = m.video.availableAudioTracks
    if tracks = invalid or tracks.Count() < 2 then return
    order = []
    pref = PrefGet("audioLang", "")
    if pref <> "" then order.Push(pref)
    order.Append(["spa", "eng"])
    pick = ""
    for each want in order
        for each t in tracks
            if pick = "" and SubLangKey(MB_TrackLang(t)) = want then pick = t.Track
        end for
    end for
    if pick = "" then
        for each t in tracks
            if pick = "" and SubLangKey(MB_TrackLang(t)) <> "hin" then pick = t.Track
        end for
    end if
    if pick <> "" and pick <> m.video.currentAudioTrack then m.video.audioTrack = pick
end sub

function MB_TrackLang(track as Object) as String
    if track.Language = invalid then return ""
    return track.Language
end function

function currentAudio(tracks as Object) as Dynamic
    for each t in tracks
        if t.Track = m.video.currentAudioTrack then return t
    end for
    return tracks[0]
end function

function audioTrackName(track as Dynamic) as String
    if track = invalid then return "Default"
    name = SubLangName(SubLangKey(MB_TrackLang(track)))
    if MB_TrackLang(track) = "" then name = "Track"
    if track.Name <> invalid and track.Name <> "" and LCase(track.Name) <> LCase(name) then name = name + " (" + track.Name + ")"
    return name
end function

sub openAudio()
    tracks = m.video.availableAudioTracks
    if tracks = invalid then return
    names = []
    current = 0
    for i = 0 to tracks.Count() - 1
        names.Push(audioTrackName(tracks[i]))
        if tracks[i].Track = m.video.currentAudioTrack then current = i
    end for
    m.dialog.tag = "audio"
    m.dialog.title = "Audio"
    m.dialog.message = "Your choice is remembered for next time."
    m.dialog.currentIndex = current
    m.dialog.options = names
    m.dialog.visible = true
    m.dialog.setFocus(true)
end sub

sub showToast(text as String)
    m.toastText.text = text
    m.toastBg.width = m.toastText.boundingRect().width + 56
    m.toast.visible = true
    m.toastTimer.control = "stop"
    m.toastTimer.control = "start"
end sub

sub hideToast()
    m.toast.visible = false
end sub

sub close()
    if m.closing then return
    saveProgress(false, true)
    m.closing = true
    m.video.control = "stop"
    m.top.action = { type: "close" }
end sub

sub onFocusIn()
    if m.dialog.visible then
        m.dialog.setFocus(true)
    else
        m.video.setFocus(true)
    end if
end sub

function onKeyEvent(key as String, press as Boolean) as Boolean
    if not press then return false
    if key = "back" then
        close()
        return true
    else if key = "options" then
        openOptions()
        return true
    end if
    return false
end function
