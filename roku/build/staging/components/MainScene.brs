sub init()
    m.top.backgroundColor = "0x0A0E14FF"
    m.top.backgroundUri = ""
    m.sectionsGroup = m.top.findNode("sections")
    m.stackGroup = m.top.findNode("stack")
    m.nav = m.top.findNode("nav")
    m.exitDialog = m.top.findNode("exitDialog")

    m.sections = [
        { title: "Home", kind: "browse", tabId: "1" }
        { title: "Movies", kind: "browse", tabId: "2" }
        { title: "TV Shows", kind: "browse", tabId: "5" }
        { title: "Anime", kind: "browse", tabId: "8" }
        { title: "Search", kind: "search", tabId: "" }
        { title: "My Library", kind: "browse", tabId: "library" }
        { title: "Providers", kind: "providers", tabId: "" }
        { title: "Settings", kind: "settings", tabId: "" }
    ]
    titles = []
    for each s in m.sections
        titles.Push(s.title)
    end for
    m.nav.items = titles
    m.nav.observeField("chosen", "onNavChosen")
    m.nav.observeField("highlightIndex", "onNavHighlight")

    m.exitDialog.observeField("selected", "onExitSelected")
    m.exitDialog.observeField("dismissed", "onExitDismissed")

    m.navTimer = CreateObject("roSGNode", "Timer")
    m.navTimer.duration = 0.45
    m.navTimer.repeat = false
    m.navTimer.observeField("fire", "onNavTimer")

    m.sectionScreens = {}
    m.stack = []
    m.current = -1
    showSection(0, true)
end sub

' ---------------------------------------------------------------------------
' Sections
' ---------------------------------------------------------------------------

sub showSection(index as Integer, focusContent as Boolean)
    if index = m.current then
        if focusContent then focusSection()
        return
    end if
    if m.current >= 0 then
        old = sectionScreen(m.current)
        old.visible = false
    end if
    m.current = index
    m.nav.selectedIndex = index
    screen = sectionScreen(index)
    screen.visible = true
    screen.refresh = true
    if focusContent then focusSection()
end sub

function sectionScreen(index as Integer) as Object
    key = "s" + index.ToStr()
    screen = m.sectionScreens[key]
    if screen <> invalid then return screen
    section = m.sections[index]
    if section.kind = "search" then
        screen = m.sectionsGroup.createChild("SearchScreen")
    else if section.kind = "providers" then
        screen = m.sectionsGroup.createChild("ProvidersScreen")
    else if section.kind = "settings" then
        screen = m.sectionsGroup.createChild("SettingsScreen")
    else
        screen = m.sectionsGroup.createChild("BrowseScreen")
        screen.tabId = section.tabId
    end if
    screen.observeField("action", "onScreenAction")
    m.sectionScreens[key] = screen
    return screen
end function

sub focusSection()
    sectionScreen(m.current).focusIn = true
end sub

sub onNavChosen()
    m.navTimer.control = "stop"
    showSection(m.nav.chosen, true)
end sub

sub onNavHighlight()
    if m.nav.hasFocus() and m.nav.highlightIndex <> m.current then
        m.navTimer.control = "stop"
        m.navTimer.control = "start"
    end if
end sub

sub onNavTimer()
    if m.nav.hasFocus() then showSection(m.nav.highlightIndex, false)
end sub

' ---------------------------------------------------------------------------
' Screen stack
' ---------------------------------------------------------------------------

sub pushScreen(screen as Object)
    if m.stack.Count() > 0 then
        m.stack.Peek().visible = false
    else
        m.sectionsGroup.visible = false
        m.nav.visible = false
    end if
    m.stackGroup.appendChild(screen)
    screen.observeField("action", "onScreenAction")
    m.stack.Push(screen)
    screen.focusIn = true
end sub

sub popScreen()
    if m.stack.Count() = 0 then return
    top = m.stack.Pop()
    top.unobserveField("action")
    m.stackGroup.removeChild(top)
    if m.stack.Count() > 0 then
        prev = m.stack.Peek()
        prev.visible = true
        prev.refresh = true
        prev.focusIn = true
    else
        m.sectionsGroup.visible = true
        m.nav.visible = true
        screen = sectionScreen(m.current)
        screen.refresh = true
        screen.focusIn = true
    end if
end sub

sub onScreenAction(event as Object)
    action = event.getData()
    if action = invalid or action.type = invalid then return
    t = action.type
    #if DEBUG
        print "[scene] action "; t
    #end if
    if t = "details" then
        openDetails(action.item)
    else if t = "episodes" then
        screen = CreateObject("roSGNode", "EpisodesScreen")
        screen.item = action.item
        screen.seasons = action.seasons
        screen.startSeason = action.se
        screen.startEpisode = action.ep
        screen.ready = true
        pushScreen(screen)
    else if t = "play" then
        screen = CreateObject("roSGNode", "PlayerScreen")
        screen.item = action.item
        screen.seasons = action.seasons
        screen.se = action.se
        screen.ep = action.ep
        screen.resume = action.resume
        ' Explicit comparison: the old chained `a = b = true` form read as a bug.
        screen.altMode = (action.altMode = true)
        pushScreen(screen)
        screen.start = true
    else if t = "close" then
        popScreen()
    else if t = "itemOptions" then
        openItemOptions(action.item, action.library = true)
    end if
end sub

sub openDetails(item as Object)
    if item = invalid then return
    screen = CreateObject("roSGNode", "DetailsScreen")
    ' Details may switch audio versions in place, so never mutate the row's node.
    screen.item = item.clone(false)
    pushScreen(screen)
end sub

' Options (*) on a tile: quick add/remove from My List or Continue Watching.
sub openItemOptions(item as Object, inLibrary as Boolean)
    m.optionsItem = item
    m.dialogReturnToNav = false
    options = []
    m.optionActions = []
    options.Push("Open details")
    m.optionActions.Push("details")
    if FavoriteHas(item.sid) then
        options.Push("Remove from My List")
    else
        options.Push("Add to My List")
    end if
    m.optionActions.Push("fav")
    if HistoryFind(item.sid) <> invalid then
        options.Push("Remove from watch history")
        m.optionActions.Push("history")
    end if
    m.exitDialog.tag = "item"
    m.exitDialog.title = item.title
    m.exitDialog.message = ""
    m.exitDialog.currentIndex = -1
    m.exitDialog.options = options
    m.exitDialog.visible = true
    m.exitDialog.setFocus(true)
end sub

' ---------------------------------------------------------------------------
' Dialog (exit confirmation + tile options)
' ---------------------------------------------------------------------------

sub showExitDialog()
    m.exitDialog.tag = "exit"
    m.exitDialog.title = "Exit MovieBox?"
    m.exitDialog.message = ""
    m.exitDialog.currentIndex = -1
    m.exitDialog.options = ["Exit", "Cancel"]
    m.exitDialog.visible = true
    m.exitDialog.setFocus(true)
end sub

sub hideDialog()
    m.exitDialog.visible = false
    if m.stack.Count() > 0 then
        m.stack.Peek().focusIn = true
    else if m.dialogReturnToNav = true then
        m.nav.setFocus(true)
    else
        focusSection()
    end if
end sub

sub onExitDismissed()
    hideDialog()
end sub

sub onExitSelected()
    tag = m.exitDialog.tag
    index = m.exitDialog.selected
    if tag = "exit" then
        if index = 0 then
            m.top.exitApp = true
        else
            hideDialog()
        end if
        return
    end if
    m.exitDialog.visible = false
    actionName = m.optionActions[index]
    item = m.optionsItem
    if actionName = "details" then
        openDetails(item)
        return
    else if actionName = "fav" then
        FavoriteToggle(SnapshotItem(item))
    else if actionName = "history" then
        HistoryRemove(item.sid)
    end if
    sectionScreen(m.current).refresh = true
    hideDialog()
end sub

' ---------------------------------------------------------------------------
' Deep links
' ---------------------------------------------------------------------------

sub onLaunchArgs()
    args = m.top.launchArgs
    if args = invalid or args.contentId = invalid or args.contentId = "" then return
    ' Roku Search and voice hand us a mediaType; a series must open as a series,
    ' otherwise the details screen offers a play button for a whole show.
    mediaType = ""
    if args.mediaType <> invalid then mediaType = LCase(args.mediaType)
    stype = 1
    if Instr(1, mediaType, "series") > 0 or Instr(1, mediaType, "episode") > 0 or Instr(1, mediaType, "season") > 0 then stype = 2
    node = CreateObject("roSGNode", "ContentNode")
    node.title = "Loading..."
    node.addFields({ sid: args.contentId, stype: stype, tag: "", year: "", genre: "", rating: "", durationText: "", country: "", backdrop: "", wide: "", progress: 0.0, se: 0, ep: 0, resumeAt: 0, subtitle: "" })
    while m.stack.Count() > 0
        popScreen()
    end while
    openDetails(node)
end sub

' ---------------------------------------------------------------------------
' Keys
' ---------------------------------------------------------------------------

function onKeyEvent(key as String, press as Boolean) as Boolean
    #if DEBUG
        print "[scene] key "; key; " press="; press; " stack="; m.stack.Count()
    #end if
    if not press then return false
    if m.exitDialog.visible then return true

    if m.stack.Count() > 0 then
        if key = "back" then
            popScreen()
            return true
        end if
        return false
    end if

    if key = "up" and not m.nav.hasFocus() then
        m.nav.setFocus(true)
        return true
    else if key = "down" and m.nav.hasFocus() then
        m.navTimer.control = "stop"
        if m.nav.highlightIndex <> m.current then showSection(m.nav.highlightIndex, false)
        focusSection()
        return true
    else if key = "back" then
        if not m.nav.hasFocus() then
            m.nav.setFocus(true)
        else if m.current <> 0 then
            showSection(0, false)
            m.nav.selectedIndex = 0
        else
            m.dialogReturnToNav = true
            showExitDialog()
        end if
        return true
    end if
    return false
end function
