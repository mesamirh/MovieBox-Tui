sub init()
    m.frame = m.top.findNode("frame")
    m.base = m.top.findNode("base")
    m.fallback = m.top.findNode("fallback")
    m.image = m.top.findNode("image")
    m.shade = m.top.findNode("shade")
    m.wideTitle = m.top.findNode("wideTitle")
    m.badge = m.top.findNode("badge")
    m.badgeBg = m.top.findNode("badgeBg")
    m.badgeText = m.top.findNode("badgeText")
    m.progressGroup = m.top.findNode("progressGroup")
    m.progressTrack = m.top.findNode("progressTrack")
    m.progressFill = m.top.findNode("progressFill")
    m.caption = m.top.findNode("caption")
    m.subCaption = m.top.findNode("subCaption")
    m.image.observeField("loadStatus", "onImageStatus")
    m.isWide = false
end sub

sub onSizeChange()
    layout()
end sub

sub layout()
    w = m.top.width
    h = m.top.height
    if w <= 0 or h <= 0 then return
    m.isWide = w > h
    artH = h
    ' Grid tiles reserve room under the artwork for a caption.
    if not m.isWide and h > w * 1.62 then artH = Int(w * 1.5)
    m.artH = artH

    border = 5
    m.frame.translation = [-border, -border]
    m.frame.width = w + border * 2
    m.frame.height = artH + border * 2
    m.base.width = w
    m.base.height = artH
    m.fallback.width = w - 24
    m.fallback.height = artH
    m.fallback.translation = [12, 0]
    m.image.width = w
    m.image.height = artH
    m.image.loadWidth = w
    m.image.loadHeight = artH

    m.shade.width = w
    m.shade.height = artH
    m.wideTitle.translation = [20, artH - 54]
    m.wideTitle.width = w - 40

    m.progressGroup.translation = [0, artH - 8]
    m.progressTrack.width = w

    m.caption.translation = [0, artH + 14]
    m.caption.width = w
    m.subCaption.translation = [0, artH + 44]
    m.subCaption.width = w
    m.caption.visible = (artH < h)
    m.subCaption.visible = (artH < h)
    applyContent()
end sub

sub onContentChange()
    applyContent()
end sub

sub applyContent()
    c = m.top.itemContent
    if c = invalid then return
    w = m.top.width
    if w <= 0 then return

    m.fallback.text = c.title
    uri = c.HDPosterUrl
    if m.isWide then
        if c.wide <> invalid and c.wide <> "" then
            uri = c.wide
        else if c.backdrop <> invalid and c.backdrop <> "" then
            uri = c.backdrop
        end if
    end if
    if m.image.uri <> uri then
        m.image.opacity = 0
        m.image.uri = uri
    end if

    ' Landscape artwork already carries the title logo, so no overlay text.
    m.shade.visible = false
    m.wideTitle.visible = false

    tagText = ""
    if c.tag <> invalid then tagText = c.tag
    if c.subtitle <> invalid and c.subtitle <> "" and m.isWide = false and m.caption.visible = false then tagText = c.subtitle
    if tagText <> "" then
        m.badgeText.text = UCase(tagText)
        ' Estimate instead of measuring: recycled list items report stale bounds.
        textW = Int(Len(tagText) * 12.5) + 4
        if textW > w - 40 then textW = w - 40
        m.badgeText.width = textW
        m.badgeBg.width = textW + 20
        m.badge.translation = [10, 10]
        m.badge.visible = true
    else
        m.badge.visible = false
    end if

    progress = 0.0
    if c.progress <> invalid then progress = c.progress
    if progress > 0.01 then
        if progress > 1 then progress = 1
        m.progressFill.width = Int(w * progress)
        m.progressGroup.visible = true
    else
        m.progressGroup.visible = false
    end if

    m.caption.text = c.title
    sub2 = ""
    if c.subtitle <> invalid and c.subtitle <> "" then
        sub2 = c.subtitle
    else
        kind = "Movie"
        if c.stype = 2 then kind = "Series"
        sub2 = kind
        if c.year <> invalid and c.year <> "" then sub2 = c.year + "  •  " + kind
    end if
    m.subCaption.text = sub2
end sub

sub onImageStatus()
    if m.image.loadStatus = "ready" then
        m.image.opacity = 1
        m.fallback.visible = false
    else if m.image.loadStatus = "failed" then
        m.image.opacity = 0
        m.fallback.visible = true
    else
        m.fallback.visible = true
    end if
end sub

sub onFocusChange()
    listFocused = m.top.rowHasFocus or m.top.gridHasFocus
    pct = m.top.focusPercent
    if not listFocused then pct = 0
    m.frame.opacity = pct
    if pct > 0.5 then
        m.caption.color = "0xF5B82EFF"
    else
        m.caption.color = "0xF2F4F8FF"
    end if
end sub
