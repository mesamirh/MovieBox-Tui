sub init()
    m.panel = m.top.findNode("panel")
    m.panelBg = m.top.findNode("panelBg")
    m.titleLabel = m.top.findNode("titleLabel")
    m.messageLabel = m.top.findNode("messageLabel")
    m.list = m.top.findNode("list")
    m.hint = m.top.findNode("hint")
    m.list.observeField("itemSelected", "onItemSelected")
    m.top.observeField("focusedChild", "onFocusedChild")
end sub

sub onFocusedChild()
    if m.top.visible and m.top.hasFocus() then m.list.setFocus(true)
end sub

sub onTitleChange()
    m.titleLabel.text = m.top.title
    m.messageLabel.text = m.top.message
    layout()
end sub

sub onOptionsChange()
    content = CreateObject("roSGNode", "ContentNode")
    options = m.top.options
    if options = invalid then options = []
    for i = 0 to options.Count() - 1
        child = content.createChild("ContentNode")
        text = "    " + options[i]
        if i = m.top.currentIndex then text = text + "      (current)"
        child.title = text
    end for
    m.list.content = content
    if m.top.currentIndex >= 0 and m.top.currentIndex < options.Count() then
        m.list.jumpToItem = m.top.currentIndex
    end if
    layout()
end sub

sub layout()
    top = 170
    if m.top.message <> invalid and m.top.message <> "" then
        top = 108 + m.messageLabel.boundingRect().height + 32
    end if
    m.list.translation = [40, top]
    count = 0
    if m.top.options <> invalid then count = m.top.options.Count()
    rows = count
    if rows > 7 then rows = 7
    if rows < 1 then rows = 1
    listH = rows * 80
    m.hint.translation = [56, top + listH + 16]
    panelH = top + listH + 72
    m.panelBg.height = panelH
    m.panel.translation = [(1920 - 820) / 2, (1080 - panelH) / 2]
end sub

sub onItemSelected()
    m.top.selected = m.list.itemSelected
end sub

function onKeyEvent(key as String, press as Boolean) as Boolean
    ' A hidden dialog must let keys bubble to its parent screen.
    if not m.top.visible then return false
    if not press then return false
    if key = "back" then
        m.top.dismissed = true
        return true
    end if
    ' Swallow everything else so focus never leaks behind the modal.
    return key <> "home"
end function
