sub init()
    m.bg = m.top.findNode("bg")
    m.label = m.top.findNode("label")
    render()
end sub

sub layout()
    m.label.text = m.top.text
    w = m.label.boundingRect().width + 72
    if w < 200 then w = 200
    m.label.width = w
    m.bg.width = w
    m.top.buttonWidth = w
    render()
end sub

sub render()
    if m.top.highlighted then
        m.bg.blendColor = "0xF5B82EFF"
        m.label.color = "0x0A0E14FF"
    else if m.top.primary then
        m.bg.blendColor = "0xF2F4F8FF"
        m.label.color = "0x0A0E14FF"
    else
        m.bg.blendColor = "0xFFFFFF2E"
        m.label.color = "0xF2F4F8FF"
    end if
end sub
