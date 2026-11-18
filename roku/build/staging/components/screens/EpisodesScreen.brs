sub init()
    m.backdrop = m.top.findNode("backdrop")
    m.titleLabel = m.top.findNode("title")
    m.subtitle = m.top.findNode("subtitle")
    m.seasonList = m.top.findNode("seasonList")
    m.grid = m.top.findNode("grid")
    m.episodesHeader = m.top.findNode("episodesHeader")
    m.seasonList.observeField("itemFocused", "onSeasonFocused")
    m.seasonList.observeField("itemSelected", "onSeasonSelected")
    m.grid.observeField("itemSelected", "onEpisodeSelected")
    m.currentSeasonIndex = -1
    m.pendingEpisode = 0
end sub

sub onReady()
    item = m.top.item
    if item = invalid then return
    m.titleLabel.text = item.title
    m.backdrop.uri = item.backdrop
    seasons = m.top.seasons
    total = 0
    content = CreateObject("roSGNode", "ContentNode")
    startIndex = 0
    for i = 0 to seasons.Count() - 1
        s = seasons[i]
        total = total + s.maxEp
        child = content.createChild("ContentNode")
        child.title = "    Season " + s.se.ToStr()
        if s.se = m.top.startSeason then startIndex = i
    end for
    seasonWord = " seasons"
    if seasons.Count() = 1 then seasonWord = " season"
    m.subtitle.text = seasons.Count().ToStr() + seasonWord + "  •  " + total.ToStr() + " episodes"
    m.seasonList.content = content
    m.pendingEpisode = m.top.startEpisode
    m.seasonList.jumpToItem = startIndex
    showSeason(startIndex)
end sub

sub onSeasonFocused()
    showSeason(m.seasonList.itemFocused)
end sub

sub onSeasonSelected()
    m.grid.setFocus(true)
end sub

sub showSeason(index as Integer)
    seasons = m.top.seasons
    if index < 0 or index >= seasons.Count() then return
    if index = m.currentSeasonIndex then return
    m.currentSeasonIndex = index
    s = seasons[index]
    m.episodesHeader.text = "SEASON " + s.se.ToStr() + "  •  " + s.maxEp.ToStr() + " EPISODES"
    buildGrid(s)
end sub

sub buildGrid(season as Object)
    item = m.top.item
    entry = HistoryFind(item.sid)
    episodes = invalid
    if entry <> invalid then episodes = entry.episodes
    content = CreateObject("roSGNode", "ContentNode")
    for ep = 1 to season.maxEp
        child = content.createChild("ContentNode")
        child.HDPosterUrl = item.backdrop
        child.addFields({ se: season.se, ep: ep, progress: 0.0, subtitle: "", resumeAt: 0 })
        if episodes <> invalid then
            info = episodes[season.se.ToStr() + "x" + ep.ToStr()]
            if info <> invalid and info.d > 0 then
                ratio = info.p / info.d
                child.progress = ratio
                if ratio >= 0.94 then
                    child.subtitle = "Watched"
                else
                    child.subtitle = FormatClock(info.d - info.p) + " left"
                    child.resumeAt = info.p
                end if
            end if
        end if
    end for
    m.grid.content = content
    if m.pendingEpisode > 0 and m.pendingEpisode <= season.maxEp then
        m.grid.jumpToItem = m.pendingEpisode - 1
    end if
    m.pendingEpisode = 0
end sub

sub onEpisodeSelected()
    child = m.grid.content.getChild(m.grid.itemSelected)
    if child = invalid then return
    m.top.action = { type: "play", item: m.top.item, se: child.se, ep: child.ep, resume: child.resumeAt, seasons: m.top.seasons }
end sub

sub onFocusIn()
    if m.grid.content <> invalid and m.grid.content.getChildCount() > 0 then
        m.grid.setFocus(true)
    else
        m.seasonList.setFocus(true)
    end if
end sub

sub onRefresh()
    index = m.currentSeasonIndex
    if index < 0 then return
    focused = m.grid.itemFocused
    m.currentSeasonIndex = -1
    m.pendingEpisode = focused + 1
    showSeason(index)
end sub

function onKeyEvent(key as String, press as Boolean) as Boolean
    if not press then return false
    if key = "right" and m.seasonList.hasFocus() then
        m.grid.setFocus(true)
        return true
    else if key = "left" and m.grid.hasFocus() then
        m.seasonList.setFocus(true)
        return true
    end if
    return false
end function
