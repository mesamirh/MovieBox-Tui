sub init()
    m.bg = m.top.findNode("bg")
    m.label = m.top.findNode("label")
end sub

sub onContentChange()
    c = m.top.itemContent
    if c = invalid then return
    m.bg.width = m.top.width
    m.bg.height = m.top.height
    m.label.width = m.top.width
    m.label.height = m.top.height
    m.label.text = c.title
    if Len(c.title) > 1 then
        m.label.font.size = 22
    else
        m.label.font.size = 30
    end if
end sub

sub onFocusChange()
    pct = m.top.focusPercent
    if not m.top.gridHasFocus then pct = 0
    if pct > 0.5 then
        m.bg.color = "0xF5B82EFF"
        m.label.color = "0x0A0E14FF"
    else
        m.bg.color = "0xFFFFFF14"
        m.label.color = "0xF2F4F8FF"
    end if
end sub
