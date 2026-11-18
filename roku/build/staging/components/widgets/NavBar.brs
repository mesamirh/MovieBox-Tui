sub init()
    m.tabs = m.top.findNode("tabs")
    m.top.focusable = true
    m.top.observeField("focusedChild", "onFocusedChild")
    m.labels = []
    m.pills = []
    m.highlight = 0
    m.brandBox = m.top.findNode("brandBox")
    m.brandMovie = m.top.findNode("brandMovie")
end sub

sub onItemsChange()
    m.tabs.removeChildrenIndex(m.tabs.getChildCount(), 0)
    m.labels = []
    m.pills = []
    x = 0
    m.brandBox.translation = [160 + m.brandMovie.boundingRect().width, 44]
    for each title in m.top.items
        pill = m.tabs.createChild("Poster")
        pill.uri = "pkg:/images/pill.9.png"
        pill.blendColor = "0xF5B82EFF"
        pill.height = 56
        pill.translation = [x, 0]
        pill.opacity = 0
        label = m.tabs.createChild("Label")
        label.text = title
        font = CreateObject("roSGNode", "Font")
        font.uri = "pkg:/fonts/Inter-SemiBold.ttf"
        font.size = 27
        label.font = font
        label.color = "0xC9D1DCFF"
        label.vertAlign = "center"
        label.height = 56
        label.translation = [x + 26, 0]
        w = label.boundingRect().width
        pill.width = w + 52
        m.labels.Push(label)
        m.pills.Push(pill)
        x = x + w + 52 + 12
    end for
    render()
end sub

sub onSelectedChange()
    m.highlight = m.top.selectedIndex
    render()
end sub

sub onFocusedChild()
    if m.top.hasFocus() then
        m.highlight = m.top.selectedIndex
    end if
    render()
end sub

sub render()
    focused = m.top.hasFocus()
    for i = 0 to m.labels.Count() - 1
        label = m.labels[i]
        pill = m.pills[i]
        if focused and i = m.highlight then
            pill.opacity = 1
            pill.blendColor = "0xF5B82EFF"
            label.color = "0x0A0E14FF"
        else if i = m.top.selectedIndex then
            pill.opacity = 1
            pill.blendColor = "0xFFFFFF26"
            label.color = "0xF2F4F8FF"
        else
            pill.opacity = 0
            label.color = "0xA9B2C0FF"
        end if
    end for
    m.top.highlightIndex = m.highlight
end sub

function onKeyEvent(key as String, press as Boolean) as Boolean
    if not press then return false
    if key = "left" then
        if m.highlight > 0 then
            m.highlight = m.highlight - 1
            render()
        end if
        return true
    else if key = "right" then
        if m.highlight < m.labels.Count() - 1 then
            m.highlight = m.highlight + 1
            render()
        end if
        return true
    else if key = "OK" then
        m.top.chosen = m.highlight
        return true
    end if
    return false
end function
