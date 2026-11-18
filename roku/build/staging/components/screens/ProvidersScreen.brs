' Providers screen: built-in scrapers (4KHDHub, Dramachi) and user-added Stremio addons.

sub init()
    m.list = m.top.findNode("list")
    m.status = m.top.findNode("status")
    m.dialog = m.top.findNode("dialog")
    m.top.findNode("tips").text = "• Use the Roku mobile app keyboard to paste or type the URL." + Chr(10) + "• stremio:// links and URLs without /manifest.json work too." + Chr(10) + "• Only direct http links play on Roku; torrent-only results are skipped." + Chr(10) + "• Results from every provider are merged, best match first."
    m.builtins = [
        { key: "4khdhub", name: "4KHDHub", about: "1080p and 4K files" }
        { key: "dramachi", name: "Dramachi", about: "light 540p files, dramas" }
    ]
    m.rows = []
    m.task = invalid
    m.list.observeField("itemSelected", "onRowSelected")
    m.dialog.observeField("selected", "onDialogSelected")
    m.dialog.observeField("dismissed", "onDialogDismissed")
    render()
end sub

sub onFocusIn()
    if m.dialog.visible then
        m.dialog.setFocus(true)
    else
        m.list.setFocus(true)
    end if
end sub

function onOff(enabled as Boolean) as String
    if enabled then return "On"
    return "Off"
end function

sub render()
    if m.list = invalid then return
    m.rows = []
    content = CreateObject("roSGNode", "ContentNode")
    for each b in m.builtins
        addRow(content, { kind: "builtin", key: b.key, name: b.name }, b.name + "   ·   Built-in, " + b.about + "   ·   " + onOff(ProviderEnabled(b.key)))
    end for
    for each a in AddonsAll()
        addRow(content, { kind: "addon", url: a.url, name: a.name, enabled: a.enabled = true }, a.name + "   ·   Stremio addon   ·   " + onOff(a.enabled = true))
    end for
    addRow(content, { kind: "add" }, "+   Add a Stremio addon...")
    focused = m.list.itemFocused
    m.list.content = content
    if focused > 0 and focused < m.rows.Count() then m.list.jumpToItem = focused
end sub

sub addRow(content as Object, row as Object, text as String)
    child = content.createChild("ContentNode")
    child.title = "    " + text
    m.rows.Push(row)
end sub

sub onRowSelected()
    row = m.rows[m.list.itemSelected]
    m.selectedRow = row
    if row.kind = "builtin" then
        ProviderSetEnabled(row.key, not ProviderEnabled(row.key))
        setStatus(row.name + " turned " + LCase(onOff(ProviderEnabled(row.key))) + ".")
        render()
    else if row.kind = "addon" then
        toggle = "Turn off"
        if not row.enabled then toggle = "Turn on"
        m.dialog.tag = "addon"
        m.dialog.title = row.name
        m.dialog.message = row.url
        m.dialog.currentIndex = -1
        m.dialog.options = [toggle, "Remove", "Cancel"]
        m.dialog.visible = true
        m.dialog.setFocus(true)
    else
        openKeyboard("", "")
    end if
end sub

sub onDialogSelected()
    index = m.dialog.selected
    row = m.selectedRow
    closeDialog()
    if m.dialog.tag <> "addon" or row = invalid then return
    if index = 0 then
        AddonsSetEnabled(row.url, not row.enabled)
        setStatus(row.name + " turned " + LCase(onOff(not row.enabled)) + ".")
    else if index = 1 then
        AddonsRemove(row.url)
        setStatus(row.name + " removed.")
    end if
    render()
end sub

sub onDialogDismissed()
    closeDialog()
end sub

sub closeDialog()
    m.dialog.visible = false
    m.list.setFocus(true)
end sub

sub setStatus(text as String)
    m.status.text = text
end sub

' ---------------------------------------------------------------------------
' Adding an addon
' ---------------------------------------------------------------------------

sub openKeyboard(text as String, problem as String)
    message = "Paste or type the addon's manifest URL."
    if problem <> "" then message = problem
    ' StandardKeyboardDialog needs Roku OS 10; older devices get the classic dialog.
    kb = CreateObject("roSGNode", "StandardKeyboardDialog")
    if kb = invalid then
        kb = CreateObject("roSGNode", "KeyboardDialog")
        kb.text = text
        kb.message = message
    else
        kb.textEditBox.text = text
        kb.message = [message]
    end if
    kb.title = "Add a Stremio addon"
    kb.buttons = ["Add", "Cancel"]
    kb.observeField("buttonSelected", "onKeyboardButton")
    kb.observeField("wasClosed", "onKeyboardClosed")
    m.keyboard = kb
    m.top.getScene().dialog = kb
end sub

function keyboardText() as String
    kb = m.keyboard
    if kb.textEditBox <> invalid then return kb.textEditBox.text
    return kb.text
end function

sub onKeyboardButton()
    kb = m.keyboard
    if kb = invalid then return
    text = keyboardText()
    add = kb.buttonSelected = 0
    kb.close = true
    m.keyboard = invalid
    m.list.setFocus(true)
    if add and text.Trim() <> "" then checkAddon(text)
end sub

sub onKeyboardClosed()
    m.keyboard = invalid
    m.list.setFocus(true)
end sub

sub checkAddon(url as String)
    setStatus("Checking the addon...")
    m.lastUrl = url
    m.task = CreateObject("roSGNode", "ApiTask")
    m.task.observeField("response", "onManifest")
    m.task.request = { action: "addonmanifest", url: url }
    m.task.control = "run"
end sub

sub onManifest(event as Object)
    res = event.getRoSGNode().response
    m.task = invalid
    if res.ok = true then
        if AddonsAdd(res.addon) then
            setStatus(res.addon.name + " added. It will be searched together with the other providers.")
        else
            setStatus(res.addon.name + " was already added; its settings were refreshed.")
        end if
        render()
        ' Focus the new addon (last row before "Add").
        m.list.jumpToItem = m.rows.Count() - 2
    else
        setStatus(res.error)
        ' Reopen the keyboard with the same text so a typo can be fixed.
        openKeyboard(m.lastUrl, res.error)
    end if
end sub
