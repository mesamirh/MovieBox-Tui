sub init()
    m.frame = m.top.findNode("frame")
    m.bg = m.top.findNode("bg")
    m.art = m.top.findNode("art")
    m.number = m.top.findNode("number")
    m.title = m.top.findNode("title")
    m.status = m.top.findNode("status")
    m.track = m.top.findNode("track")
    m.fill = m.top.findNode("fill")
end sub

sub onContentChange()
    c = m.top.itemContent
    w = m.top.width
    h = m.top.height
    if c = invalid or w <= 0 then return
    m.frame.translation = [-5, -5]
    m.frame.width = w + 10
    m.frame.height = h + 10
    m.bg.width = w
    m.bg.height = h
    m.art.width = w
    m.art.height = h
    m.art.loadWidth = w
    m.art.loadHeight = h
    if m.art.uri <> c.HDPosterUrl then m.art.uri = c.HDPosterUrl
    m.number.text = Right("0" + c.ep.ToStr(), 2)
    m.title.text = "Episode " + c.ep.ToStr()
    m.title.translation = [24, h - 48]
    m.title.width = w - 140
    m.status.translation = [w - 224, h - 44]
    m.status.width = 200
    m.status.text = c.subtitle
    m.track.translation = [0, h - 6]
    m.track.width = w
    m.fill.translation = [0, h - 6]
    if c.progress > 0.01 then
        p = c.progress
        if p > 1 then p = 1
        m.fill.width = Int(w * p)
        m.track.visible = true
        m.fill.visible = true
    else
        m.track.visible = false
        m.fill.visible = false
    end if
end sub

sub onFocusChange()
    pct = m.top.focusPercent
    if not m.top.gridHasFocus then pct = 0
    m.frame.opacity = pct
    if pct > 0.5 then
        m.bg.color = "0x222A37FF"
        m.number.color = "0xF5B82EFF"
    else
        m.bg.color = "0x161C26FF"
        m.number.color = "0xF2F4F8FF"
    end if
end sub
