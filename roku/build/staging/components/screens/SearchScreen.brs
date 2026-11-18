sub init()
    m.queryLabel = m.top.findNode("queryLabel")
    m.keyboard = m.top.findNode("keyboard")
    m.results = m.top.findNode("results")
    m.resultsTitle = m.top.findNode("resultsTitle")
    m.resultsHint = m.top.findNode("resultsHint")
    m.spinner = m.top.findNode("spinner")
    m.debounce = m.top.findNode("debounce")

    m.query = ""
    m.lastSearched = ""
    m.page = 1
    m.hasMore = false
    m.loadingMore = false
    m.requestId = 0

    keys = []
    for each ch in "abcdefghijklmnopqrstuvwxyz1234567890".Split("")
        keys.Push(ch)
    end for
    keys.Push("SPACE")
    keys.Push("DELETE")
    keys.Push("CLEAR")
    content = CreateObject("roSGNode", "ContentNode")
    for each k in keys
        child = content.createChild("ContentNode")
        child.title = k
    end for
    m.keyboard.content = content

    m.keyboard.observeField("itemSelected", "onKeySelected")
    m.results.observeField("itemSelected", "onResultSelected")
    m.results.observeField("itemFocused", "onResultFocused")
    m.debounce.observeField("fire", "runSearch")
    renderQuery()
    showIdle()
end sub

sub showIdle()
    m.resultsTitle.text = "Search MovieBox"
    m.resultsHint.text = "Find movies, TV shows and anime by title. Results appear as you type."
    m.results.content = CreateObject("roSGNode", "ContentNode")
end sub

sub renderQuery()
    if m.query = "" then
        m.queryLabel.text = "Type a title..."
        m.queryLabel.color = "0x5A6476FF"
    else
        m.queryLabel.text = m.query + "_"
        m.queryLabel.color = "0xF2F4F8FF"
    end if
end sub

sub onKeySelected()
    key = m.keyboard.content.getChild(m.keyboard.itemSelected).title
    if key = "SPACE" then
        if m.query <> "" and Right(m.query, 1) <> " " then m.query = m.query + " "
    else if key = "DELETE" then
        if Len(m.query) > 0 then m.query = Left(m.query, Len(m.query) - 1)
    else if key = "CLEAR" then
        m.query = ""
    else
        if Len(m.query) < 40 then m.query = m.query + key
    end if
    queryChanged()
end sub

sub queryChanged()
    renderQuery()
    m.debounce.control = "stop"
    if m.query.Trim() = "" then
        m.requestId = m.requestId + 1
        m.spinner.running = false
        m.lastSearched = ""
        showIdle()
        return
    end if
    m.debounce.control = "start"
end sub

sub runSearch()
    q = m.query.Trim()
    if q = "" or q = m.lastSearched then return
    m.lastSearched = q
    m.page = 1
    m.hasMore = false
    startRequest(q, 1)
    m.resultsTitle.text = "Results for " + Chr(34) + q + Chr(34)
    m.resultsHint.text = ""
end sub

sub startRequest(q as String, page as Integer)
    m.requestId = m.requestId + 1
    if page = 1 then
        m.spinner.running = true
        m.results.content = CreateObject("roSGNode", "ContentNode")
    end if
    m.task = CreateObject("roSGNode", "ApiTask")
    m.task.observeField("response", "onSearchResponse")
    m.task.request = { action: "search", query: q, page: page, token: m.requestId }
    m.task.control = "run"
end sub

sub onSearchResponse(event as Object)
    task = event.getRoSGNode()
    res = task.response
    if res.token <> m.requestId then return
    m.spinner.running = false
    m.loadingMore = false
    if res.ok <> true then
        if res.page = invalid or res.page <= 1 then
            m.resultsHint.text = res.error
        end if
        return
    end if
    m.hasMore = res.hasMore = true
    m.page = res.page
    incoming = task.content
    if res.page <= 1 then
        m.results.content = incoming
        if res.count = 0 then
            m.resultsHint.text = "No movies or shows matched. Try another spelling."
        else
            m.resultsHint.text = ""
        end if
    else if incoming <> invalid then
        existing = m.results.content
        seen = {}
        for i = 0 to existing.getChildCount() - 1
            seen[existing.getChild(i).sid] = true
        end for
        kids = []
        for i = 0 to incoming.getChildCount() - 1
            kids.Push(incoming.getChild(i))
        end for
        for each kid in kids
            if seen[kid.sid] = invalid then existing.appendChild(kid)
        end for
    end if
end sub

sub onResultFocused()
    content = m.results.content
    if content = invalid then return
    if m.hasMore and not m.loadingMore and m.results.itemFocused >= content.getChildCount() - 10 then
        m.loadingMore = true
        startRequest(m.lastSearched, m.page + 1)
    end if
end sub

sub onResultSelected()
    item = m.results.content.getChild(m.results.itemSelected)
    if item <> invalid then m.top.action = { type: "details", item: item }
end sub

sub onFocusIn()
    if m.results.hasFocus() then return
    m.keyboard.setFocus(true)
end sub

function hasResults() as Boolean
    return m.results.content <> invalid and m.results.content.getChildCount() > 0
end function

function onKeyEvent(key as String, press as Boolean) as Boolean
    if not press then return false
    if Left(key, 4) = "Lit_" then
        ch = LCase(Mid(key, 5))
        if ch = "%20" or ch = "+" then ch = " "
        if Len(m.query) < 40 and Len(ch) = 1 then
            m.query = m.query + ch
            queryChanged()
        end if
        return true
    end if
    if key = "backspace" then
        if Len(m.query) > 0 then
            m.query = Left(m.query, Len(m.query) - 1)
            queryChanged()
        end if
        return true
    end if
    if key = "right" and m.keyboard.hasFocus() then
        if hasResults() then m.results.setFocus(true)
        return true
    else if key = "left" and m.results.hasFocus() then
        m.keyboard.setFocus(true)
        return true
    else if key = "back" and m.results.hasFocus() then
        m.keyboard.setFocus(true)
        return true
    else if key = "replay" then
        if Len(m.query) > 0 then
            m.query = Left(m.query, Len(m.query) - 1)
            queryChanged()
        end if
        return true
    end if
    return false
end function
