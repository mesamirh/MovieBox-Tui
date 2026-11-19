sub init()
    m.rows = m.top.findNode("rows")
    m.spinner = m.top.findNode("spinner")
    m.backdrop = m.top.findNode("backdrop")
    m.fadeIn = m.top.findNode("fadeIn")
    m.heroTimer = m.top.findNode("heroTimer")
    m.hero = m.top.findNode("hero")
    m.heroTag = m.top.findNode("heroTag")
    m.heroTagBg = m.top.findNode("heroTagBg")
    m.heroTagText = m.top.findNode("heroTagText")
    m.heroTitle = m.top.findNode("heroTitle")
    m.heroMeta = m.top.findNode("heroMeta")
    m.heroDesc = m.top.findNode("heroDesc")
    m.emptyState = m.top.findNode("emptyState")
    m.emptyTitle = m.top.findNode("emptyTitle")
    m.emptyText = m.top.findNode("emptyText")

    m.rows.observeField("rowItemFocused", "onItemFocused")
    m.rows.observeField("rowItemSelected", "onItemSelected")
    m.heroTimer.observeField("fire", "updateHero")
    m.backdrop.observeField("loadStatus", "onBackdropStatus")

    m.remoteRows = []
    m.loaded = false
    m.failed = false
    m.focusedItem = invalid
    m.lastFocus = [0, 0]
    m.top.observeField("visible", "onVisibleChange")
end sub

sub onTabChange()
    load()
end sub

function isLibrary() as Boolean
    return m.top.tabId = "library"
end function

sub load()
    m.failed = false
    m.emptyState.visible = false
    if isLibrary() then
        buildRows()
        return
    end if
    ' Abandon any in-flight page load: switching tabs quickly used to leave
    ' several tasks running, all racing to overwrite m.remoteRows.
    cancelTask()
    m.spinner.message = ""
    m.spinner.running = true
    m.task = CreateObject("roSGNode", "ApiTask")
    m.task.observeField("response", "onResponse")
    m.task.request = { action: "home", tabId: m.top.tabId }
    m.task.control = "run"
end sub

sub cancelTask()
    if m.task = invalid then return
    m.task.unobserveField("response")
    m.task.control = "stop"
    m.task = invalid
end sub

sub onResponse()
    ' A cancelled load must never be able to publish its rows.
    if m.task = invalid then return
    res = m.task.response
    m.spinner.running = false
    if res.ok = true then
        m.remoteRows = []
        content = m.task.content
        if content <> invalid then
            for i = 0 to content.getChildCount() - 1
                m.remoteRows.Push(content.getChild(i))
            end for
        end if
        m.loaded = true
        buildRows()
    else
        m.failed = true
        showEmpty("Couldn't load this page", res.error + chr(10) + "Press OK to try again.")
    end if
    m.task = invalid
end sub

sub showEmpty(title as String, text as String)
    m.rows.visible = false
    m.emptyTitle.text = title
    m.emptyText.text = text
    m.emptyState.visible = true
    m.heroTitle.text = ""
    m.heroMeta.text = ""
    m.heroDesc.text = ""
    m.heroTag.visible = false
end sub

' Builds the RowList content, merging local rows (continue watching, my list).
sub buildRows()
    root = CreateObject("roSGNode", "ContentNode")
    localRows = []

    continueRow = makeContinueRow()
    if continueRow <> invalid and (isLibrary() or m.top.tabId = "1") then localRows.Push(continueRow)
    if isLibrary() then
        favRow = makeFavoritesRow()
        if favRow <> invalid then localRows.Push(favRow)
        recentRow = makeRecentRow()
        if recentRow <> invalid then localRows.Push(recentRow)
    end if

    remoteRows = m.remoteRows

    ' Keep "Featured" first, then continue watching, then the rest.
    ordered = []
    startIdx = 0
    if remoteRows.Count() > 0 and remoteRows[0].kind = "wide" then
        ordered.Push(remoteRows[0])
        startIdx = 1
    end if
    for each row in localRows
        ordered.Push(row)
    end for
    for i = startIdx to remoteRows.Count() - 1
        ordered.Push(remoteRows[i])
    end for

    if ordered.Count() = 0 then
        if isLibrary() then
            showEmpty("Your library is empty", "Titles you start watching or add to My List will show up here.")
        end if
        return
    end if

    sizes = []
    for each row in ordered
        if row.getParent() <> invalid then row.getParent().removeChild(row)
        root.appendChild(row)
        ' Uniform row height keeps vertical navigation predictable; featured tiles go 16:9.
        if row.kind = "wide" then
            sizes.Push([571, 321])
        else
            sizes.Push([214, 321])
        end if
    end for

    m.rows.rowItemSize = sizes
    m.rows.content = root
    m.rows.visible = true
    m.emptyState.visible = false

    row = m.lastFocus[0]
    col = m.lastFocus[1]
    if row >= root.getChildCount() then row = 0
    if row < root.getChildCount() and col >= root.getChild(row).getChildCount() then col = 0
    if row <> 0 or col <> 0 then m.rows.jumpToRowItem = [row, col]
    focusItem(row, col)
    ' Only take focus if this screen still owns it; a pushed screen (details, player)
    ' must never lose focus to a page that finished loading in the background.
    if m.wantFocus = true and m.top.isInFocusChain() then m.rows.setFocus(true)
end sub

function newLocalRow(title as String) as Object
    row = CreateObject("roSGNode", "ContentNode")
    row.title = title
    row.addFields({ kind: "poster", isLocal: true })
    return row
end function

function makeContinueRow() as Dynamic
    row = newLocalRow("Continue Watching")
    for each entry in HistoryAll()
        if entry.item <> invalid and not IsHiddenItem(entry.item) and HistoryInProgress(entry) then
            node = NodeFromSnapshot(entry.item)
            if entry.duration > 0 then node.progress = entry.position / entry.duration
            node.se = entry.se
            node.ep = entry.ep
            node.resumeAt = entry.position
            if entry.item.stype = 2 and entry.ep > 0 then
                node.subtitle = EpisodeCode(entry.se, entry.ep)
            else if entry.duration > 0 then
                node.subtitle = FormatClock(entry.duration - entry.position) + " left"
            end if
            row.appendChild(node)
        end if
        if row.getChildCount() >= 20 then exit for
    end for
    if row.getChildCount() = 0 then return invalid
    return row
end function

function makeFavoritesRow() as Dynamic
    row = newLocalRow("My List")
    for each item in FavoritesAll()
        if not IsHiddenItem(item) then row.appendChild(NodeFromSnapshot(item))
    end for
    if row.getChildCount() = 0 then return invalid
    return row
end function

function makeRecentRow() as Dynamic
    row = newLocalRow("Recently Watched")
    for each entry in HistoryAll()
        if entry.item <> invalid and not IsHiddenItem(entry.item) then row.appendChild(NodeFromSnapshot(entry.item))
    end for
    if row.getChildCount() = 0 then return invalid
    return row
end function

sub onItemFocused()
    f = m.rows.rowItemFocused
    if f = invalid or f.Count() < 2 then return
    m.lastFocus = [f[0], f[1]]
    #if DEBUG
        print "[browse] focused "; f[0]; ","; f[1]
    #end if
    focusItem(f[0], f[1])
end sub

sub focusItem(row as Integer, col as Integer)
    content = m.rows.content
    if content = invalid or row < 0 or row >= content.getChildCount() then return
    r = content.getChild(row)
    if col < 0 or col >= r.getChildCount() then return
    m.focusedItem = r.getChild(col)
    updateHeroText()
    ' Debounce only the (expensive) backdrop image while scrolling fast.
    m.heroTimer.control = "stop"
    m.heroTimer.control = "start"
end sub

sub updateHeroText()
    item = m.focusedItem
    if item = invalid then return
    m.heroTitle.text = item.title
    m.heroMeta.text = MetaLine(item)
    m.heroDesc.text = item.description

    y = 0
    if item.tag <> invalid and item.tag <> "" then
        m.heroTagText.text = UCase(item.tag)
        m.heroTagBg.width = m.heroTagText.boundingRect().width + 28
        m.heroTag.visible = true
        y = 54
    else
        m.heroTag.visible = false
    end if
    m.heroTitle.translation = [0, y]
    titleH = m.heroTitle.boundingRect().height
    if titleH < 70 then titleH = 80
    m.heroMeta.translation = [0, y + titleH + 12]
    m.heroDesc.translation = [0, y + titleH + 60]
    ' Pull the hero up when the title wraps so the description never hits the rows.
    heroY = 196 - (y + titleH - 80) / 2
    ' Without a synopsis, sit the title closer to the rows instead of floating.
    if item.description = invalid or item.description = "" then heroY = heroY + 110
    m.hero.translation = [96, heroY]
end sub

sub updateHero()
    item = m.focusedItem
    if item = invalid then return
    uri = item.backdrop
    if uri = invalid or uri = "" then uri = item.HDPosterUrl
    if m.backdrop.uri <> uri then
        m.fadeIn.control = "stop"
        m.backdrop.opacity = 0
        m.backdrop.uri = uri
    end if
end sub

sub onBackdropStatus()
    if m.backdrop.loadStatus = "ready" then m.fadeIn.control = "start"
end sub

sub onItemSelected()
    s = m.rows.rowItemSelected
    content = m.rows.content
    if s = invalid or content = invalid then return
    item = content.getChild(s[0]).getChild(s[1])
    if item <> invalid then m.top.action = { type: "details", item: item }
end sub

sub onFocusIn()
    m.wantFocus = true
    if m.rows.visible then
        m.rows.setFocus(true)
    else
        m.top.setFocus(true)
    end if
end sub

sub onRefresh()
    if isLibrary() then
        buildRows()
    else if m.loaded and m.top.tabId = "1" then
        buildRows()
    else if m.failed then
        load()
    end if
end sub

sub onVisibleChange()
    if not m.top.visible then m.wantFocus = false
end sub

function onKeyEvent(key as String, press as Boolean) as Boolean
    if not press then return false
    if key = "OK" and m.failed then
        load()
        return true
    end if
    if key = "down" and not m.rows.hasFocus() and m.rows.visible then
        m.rows.setFocus(true)
        return true
    end if
    if key = "options" and m.focusedItem <> invalid and m.rows.hasFocus() then
        m.top.action = { type: "itemOptions", item: m.focusedItem, library: isLibrary() }
        return true
    end if
    return false
end function
