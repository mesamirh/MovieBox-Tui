sub init()
    m.backdrop = m.top.findNode("backdrop")
    m.poster = m.top.findNode("poster")
    m.tag = m.top.findNode("tag")
    m.tagBg = m.top.findNode("tagBg")
    m.tagText = m.top.findNode("tagText")
    m.titleLabel = m.top.findNode("title")
    m.meta = m.top.findNode("meta")
    m.description = m.top.findNode("description")
    m.cast = m.top.findNode("cast")
    m.buttonsGroup = m.top.findNode("buttons")
    m.status = m.top.findNode("status")
    m.spinner = m.top.findNode("spinner")
    m.dialog = m.top.findNode("dialog")
    m.fadeIn = m.top.findNode("fadeIn")
    m.backdrop.observeField("loadStatus", "onBackdropStatus")
    m.dialog.observeField("selected", "onDialogSelected")
    m.dialog.observeField("dismissed", "onDialogDismissed")
    m.top.observeField("focusedChild", "onFocusedChild")

    m.details = invalid
    m.buttons = []
    m.focusIndex = 0
end sub

sub onItemChange()
    item = m.top.item
    if item = invalid then return
    if m.loadedFor <> invalid and m.loadedFor = item.sid then return
    m.loadedFor = item.sid
    m.details = invalid
    showBasics(item)
    loadDetails(item.sid)
end sub

sub showBasics(item as Object)
    m.titleLabel.text = item.title
    m.meta.text = MetaLine(item)
    m.description.text = item.description
    m.cast.text = ""
    if item.tag <> invalid and item.tag <> "" then
        m.tagText.text = UCase(item.tag)
        m.tagBg.width = m.tagText.boundingRect().width + 28
        m.tag.visible = true
    else
        m.tag.visible = false
    end if
    if item.HDPosterUrl <> invalid then m.poster.uri = item.HDPosterUrl
    setBackdrop(item.backdrop)
    layoutInfo()
end sub

sub setBackdrop(uri as Dynamic)
    if uri = invalid or uri = "" then return
    if m.backdrop.uri = uri then return
    m.backdrop.opacity = 0
    m.backdrop.uri = uri
end sub

sub onBackdropStatus()
    if m.backdrop.loadStatus = "ready" then m.fadeIn.control = "start"
end sub

sub layoutInfo()
    y = 0
    if m.tag.visible then y = 56
    m.titleLabel.translation = [0, y]
    y = y + m.titleLabel.boundingRect().height + 14
    m.meta.translation = [0, y]
    y = y + 54
    m.description.translation = [0, y]
    descH = m.description.boundingRect().height
    if m.description.text = "" then descH = 0
    y = y + descH + 24
    m.cast.translation = [0, y]
end sub

sub loadDetails(id as String)
    clearButtons()
    m.status.text = ""
    m.spinner.running = true
    m.task = CreateObject("roSGNode", "ApiTask")
    m.task.observeField("response", "onDetails")
    m.task.request = { action: "details", id: id }
    m.task.control = "run"
end sub

sub onDetails()
    res = m.task.response
    m.task = invalid
    m.spinner.running = false
    if res.ok <> true then
        m.status.text = res.error + "   •   Press OK to retry"
        m.retry = true
        buildButtons()
        return
    end if
    m.retry = false
    d = res.item
    m.details = d
    ' Build a fresh node from the full payload instead of mutating the observed one.
    item = CreateObject("roSGNode", "ContentNode")
    item.title = d.title
    item.description = d.description
    item.HDPosterUrl = d.poster
    if d.poster = "" then item.HDPosterUrl = m.top.item.HDPosterUrl
    backdrop = d.backdrop
    if backdrop = "" then backdrop = m.top.item.backdrop
    item.addFields({ sid: d.id, stype: d.stype, tag: d.tag, year: d.year, genre: d.genre, rating: d.rating, durationText: d.duration, country: d.country, backdrop: backdrop, wide: "", progress: 0.0, se: 0, ep: 0, resumeAt: 0, subtitle: "" })
    m.item = item
    showBasics(item)
    if d.posterLarge <> "" then m.poster.uri = d.posterLarge
    if d.cast <> "" then m.cast.text = "Starring  " + d.cast
    if d.stype = 2 and d.seasons.Count() > 0 then
        total = 0
        for each s in d.seasons
            total = total + s.maxEp
        end for
        label = d.seasons.Count().ToStr() + " Seasons"
        if d.seasons.Count() = 1 then label = "1 Season"
        m.meta.text = MetaLine(item) + "   •   " + label + "  (" + total.ToStr() + " episodes)"
    end if
    layoutInfo()
    buildButtons()
end sub

sub clearButtons()
    m.buttonsGroup.removeChildrenIndex(m.buttonsGroup.getChildCount(), 0)
    m.buttons = []
end sub

sub addButton(id as String, text as String, primary as Boolean)
    b = m.buttonsGroup.createChild("ActionButton")
    b.actionId = id
    b.text = text
    b.primary = primary
    m.buttons.Push(b)
end sub

sub buildButtons()
    clearButtons()
    item = currentItem()
    if m.details = invalid then
        if m.retry = true then addButton("retry", "Try Again", true)
        m.focusIndex = 0
        renderFocus()
        layoutButtons()
        return
    end if

    entry = HistoryFind(item.sid)
    if item.stype = 2 then
        se = 1
        ep = 1
        if m.details.seasons.Count() > 0 then se = m.details.seasons[0].se
        if entry <> invalid and entry.ep > 0 then
            se = entry.se
            ep = entry.ep
            addButton("resume", "Resume  " + EpisodeCode(se, ep), true)
        else
            addButton("play", "Play  " + EpisodeCode(se, ep), true)
        end if
        m.playTarget = { se: se, ep: ep }
        addButton("episodes", "Episodes", false)
    else
        m.playTarget = { se: 0, ep: 0 }
        if entry <> invalid and HistoryInProgress(entry) then
            addButton("resume", "Resume from " + FormatClock(entry.position), true)
            addButton("restart", "Start Over", false)
        else
            addButton("play", "Play", true)
        end if
    end if

    if m.details.dubs.Count() > 1 then
        current = "Original"
        for each dub in m.details.dubs
            if dub.current then current = dub.name
        end for
        addButton("audio", "Audio: " + current, false)
    end if

    addButton("sources", "Other Sources", false)

    if FavoriteHas(item.sid) then
        addButton("fav", "In My List", false)
    else
        addButton("fav", "+ My List", false)
    end if
    layoutButtons()
    if m.focusIndex >= m.buttons.Count() then m.focusIndex = 0
    renderFocus()
end sub

sub layoutButtons()
    x = 0
    for each b in m.buttons
        b.translation = [x, 0]
        x = x + b.buttonWidth + 20
    end for
end sub

sub renderFocus()
    if not m.dialog.visible and m.dialog.isInFocusChain() then m.top.setFocus(true)
    focused = m.top.isInFocusChain() and not m.dialog.visible
    for i = 0 to m.buttons.Count() - 1
        m.buttons[i].highlighted = focused and i = m.focusIndex
    end for
end sub

sub onFocusedChild()
    renderFocus()
end sub

sub onFocusIn()
    if m.dialog.visible then
        m.dialog.setFocus(true)
    else
        m.top.setFocus(true)
    end if
    renderFocus()
end sub

sub onRefresh()
    if m.details <> invalid then buildButtons()
end sub

function currentItem() as Object
    if m.item <> invalid then return m.item
    return m.top.item
end function

function resumePosition(item as Object) as Integer
    entry = HistoryFind(item.sid)
    if entry = invalid then return 0
    if item.stype = 2 then
        key = m.playTarget.se.ToStr() + "x" + m.playTarget.ep.ToStr()
        if entry.episodes <> invalid and entry.episodes[key] <> invalid then
            e = entry.episodes[key]
            if e.d = 0 or e.p / e.d < 0.94 then return e.p
        end if
        return 0
    end if
    if HistoryInProgress(entry) then return entry.position
    return 0
end function

sub activate(id as String)
    item = currentItem()
    if id = "retry" then
        loadDetails(m.loadedFor)
    else if id = "play" then
        m.top.action = { type: "play", item: item, se: m.playTarget.se, ep: m.playTarget.ep, resume: 0, seasons: seasonsOrEmpty() }
    else if id = "resume" then
        m.top.action = { type: "play", item: item, se: m.playTarget.se, ep: m.playTarget.ep, resume: resumePosition(item), seasons: seasonsOrEmpty() }
    else if id = "sources" then
        ' Skips MovieBox and lists files from other providers right away.
        m.top.action = { type: "play", item: item, se: m.playTarget.se, ep: m.playTarget.ep, resume: resumePosition(item), seasons: seasonsOrEmpty(), altMode: true }
    else if id = "restart" then
        m.top.action = { type: "play", item: item, se: 0, ep: 0, resume: 0, seasons: [] }
    else if id = "episodes" then
        m.top.action = { type: "episodes", item: item, seasons: seasonsOrEmpty(), se: m.playTarget.se, ep: m.playTarget.ep }
    else if id = "audio" then
        names = []
        current = 0
        for i = 0 to m.details.dubs.Count() - 1
            names.Push(m.details.dubs[i].name)
            if m.details.dubs[i].current then current = i
        end for
        openDialog("audio", "Audio Language", "Each language is a separate version of this title.", names, current)
    else if id = "fav" then
        added = FavoriteToggle(SnapshotItem(item))
        if added then
            m.status.text = "Added to My List"
        else
            m.status.text = "Removed from My List"
        end if
        buildButtons()
    end if
end sub

function seasonsOrEmpty() as Object
    if m.details = invalid or m.details.seasons = invalid then return []
    return m.details.seasons
end function

sub openDialog(tag as String, title as String, message as String, options as Object, current as Integer)
    m.dialog.tag = tag
    m.dialog.title = title
    m.dialog.message = message
    m.dialog.currentIndex = current
    m.dialog.options = options
    m.dialog.visible = true
    m.dialog.setFocus(true)
    renderFocus()
end sub

sub closeDialog()
    m.dialog.visible = false
    m.top.setFocus(true)
    renderFocus()
end sub

sub onDialogDismissed()
    closeDialog()
end sub

sub onDialogSelected()
    index = m.dialog.selected
    tag = m.dialog.tag
    closeDialog()
    if tag = "audio" and m.details <> invalid then
        dub = m.details.dubs[index]
        if dub <> invalid and not dub.current then
            m.details = invalid
            m.item = invalid
            m.loadedFor = dub.id
            loadDetails(dub.id)
        end if
    end if
end sub

function onKeyEvent(key as String, press as Boolean) as Boolean
    if not press then return false
    if key = "left" then
        if m.focusIndex > 0 then
            m.focusIndex = m.focusIndex - 1
            renderFocus()
        end if
        return true
    else if key = "right" then
        if m.focusIndex < m.buttons.Count() - 1 then
            m.focusIndex = m.focusIndex + 1
            renderFocus()
        end if
        return true
    else if key = "OK" then
        if m.buttons.Count() > m.focusIndex then activate(m.buttons[m.focusIndex].actionId)
        return true
    else if key = "play" then
        if m.buttons.Count() > 0 then activate(m.buttons[0].actionId)
        return true
    end if
    return key = "up" or key = "down"
end function
