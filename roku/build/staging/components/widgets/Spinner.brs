sub init()
    m.spin = m.top.findNode("spin")
    m.label = m.top.findNode("label")
    m.top.visible = false
end sub

sub onRunningChange()
    m.top.visible = m.top.running
    if m.top.running then
        m.spin.control = "start"
    else
        m.spin.control = "stop"
    end if
end sub

sub onMessageChange()
    m.label.text = m.top.message
end sub
