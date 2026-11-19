' Settings screen: the preferences the player and the catalogue already read.
' Values are plain pref_* strings in the "library" registry section, so every
' other screen keeps reading them through PrefGet/PrefSet unchanged.

sub init()
    m.list = m.top.findNode("list")
    m.dialog = m.top.findNode("dialog")
    m.activeRow = invalid
    m.list.observeField("itemSelected", "onItemSelected")
    m.dialog.observeField("selected", "onDialogSelected")
    m.dialog.observeField("dismissed", "onDialogDismissed")
    buildItems()
    m.list.jumpToItem = 0
end sub

' One row per preference. options[] holds the stored values, labels[] what the
' viewer reads; both arrays must stay the same length.
function optionRows() as Object
    return [
        {
            key: "hideHindi", title: "Hide Hindi releases", toggle: true
            options: ["on", "off"], labels: ["On", "Off"]
        }
        {
            key: "maxRes", title: "Maximum resolution", toggle: false
            options: ["0", "1080", "720", "480"]
            labels: ["Auto (best available)", "1080p", "720p", "480p"]
        }
        {
            key: "subtitles", title: "Subtitles", toggle: false
            options: ["off", "spa", "eng"], labels: ["Off", "Spanish", "English"]
        }
        {
            key: "audioLang", title: "Audio language", toggle: false
            options: ["", "spa", "eng"], labels: ["Automatic", "Spanish", "English"]
        }
    ]
end function

function defaultFor(key as String) as String
    if key = "hideHindi" then return "on"
    if key = "maxRes" then return "0"
    if key = "subtitles" then return "off"
    return ""
end function

function indexOfOption(options as Object, value as String) as Integer
    for i = 0 to options.Count() - 1
        if options[i] = value then return i
    end for
    return -1
end function

function rowLabel(row as Object) as String
    value = PrefGet(row.key, defaultFor(row.key))
    index = indexOfOption(row.options, value)
    ' An unknown stored value falls back to the first option instead of showing blank.
    if index < 0 then index = 0
    return row.title + "   ·   " + row.labels[index]
end function

sub buildItems()
    root = CreateObject("roSGNode", "ContentNode")
    for each row in optionRows()
        node = root.createChild("ContentNode")
        node.title = rowLabel(row)
    end for
    m.list.content = root
end sub

sub onRefresh()
    buildItems()
end sub

sub onFocusIn()
    m.list.setFocus(true)
end sub

sub onItemSelected()
    rows = optionRows()
    index = m.list.itemSelected
    if index < 0 or index >= rows.Count() then return
    row = rows[index]
    if row.toggle then
        ' A toggle flips in place: no dialog for a two-state preference.
        value = PrefGet(row.key, defaultFor(row.key))
        if value = "off" then
            PrefSet(row.key, "on")
        else
            PrefSet(row.key, "off")
        end if
        buildItems()
        return
    end if
    m.activeRow = row
    m.dialog.tag = "setting"
    m.dialog.title = row.title
    m.dialog.message = "Your choice is remembered for next time."
    m.dialog.currentIndex = indexOfOption(row.options, PrefGet(row.key, defaultFor(row.key)))
    m.dialog.options = row.labels
    m.dialog.visible = true
    m.dialog.setFocus(true)
end sub

sub onDialogSelected()
    choice = m.dialog.selected
    m.dialog.visible = false
    row = m.activeRow
    m.activeRow = invalid
    if row <> invalid and choice >= 0 and choice < row.options.Count() then
        PrefSet(row.key, row.options[choice])
        buildItems()
    end if
    m.list.setFocus(true)
end sub

sub onDialogDismissed()
    m.dialog.visible = false
    m.activeRow = invalid
    m.list.setFocus(true)
end sub

function onKeyEvent(key as String, press as Boolean) as Boolean
    if not press then return false
    ' Back is owned by MainScene, which pops this screen off the stack.
    return false
end function
