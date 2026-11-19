' MovieBox API client for Roku (runs on Task threads only).
'
' Mirrors the signing scheme used by the desktop client:
'   x-tr-signature = "<ts>|2|" + base64(HMAC-MD5(secret, canonical))
'   canonical      = METHOD \n accept \n content-type \n body-length \n ts \n md5(body) \n path?sorted-query

sub MB_Log(message as String)
    #if DEBUG
        print "[moviebox] "; message
    #end if
end sub

function MB_Hosts() as Object
    return [
        "https://api6.aoneroom.com"
        "https://api5.aoneroom.com"
        "https://api4.aoneroom.com"
        "https://api4sg.aoneroom.com"
        "https://api3.aoneroom.com"
        "https://api6sg.aoneroom.com"
        "https://api.inmoviebox.com"
    ]
end function

function MB_SecretHex() as String
    return "efa891974eecd3148df63aa611602defd101259ba521022c57ae0566bd8e"
end function

function MB_CertSha256() as String
    return "03d4b741632d136bc23e2de4c6adcdb7712f1980b619c183221801730bbd0410"
end function

' ---------------------------------------------------------------------------
' Crypto helpers
' ---------------------------------------------------------------------------

function MB_Md5Hex(text as String) as String
    bytes = CreateObject("roByteArray")
    bytes.FromAsciiString(text)
    digest = CreateObject("roEVPDigest")
    digest.Setup("md5")
    return LCase(digest.Process(bytes))
end function

function MB_HmacMd5Base64(text as String) as String
    key = CreateObject("roByteArray")
    key.FromHexString(MB_SecretHex())
    hmac = CreateObject("roHMAC")
    if hmac.Setup("md5", key) <> 0 then return ""
    bytes = CreateObject("roByteArray")
    bytes.FromAsciiString(text)
    return hmac.Process(bytes).ToBase64String()
end function

function MB_RandomHex(length as Integer) as String
    digits = "0123456789abcdef"
    out = ""
    for i = 1 to length
        out = out + Mid(digits, Rnd(16), 1)
    end for
    return out
end function

function MB_Pad3(value as Integer) as String
    return Right("00" + value.ToStr(), 3)
end function

function MB_Reverse(text as String) as String
    out = ""
    for i = Len(text) to 1 step -1
        out = out + Mid(text, i, 1)
    end for
    return out
end function

' Ordinal (byte-wise) string comparison, matching Rust's BTreeMap ordering.
function MB_StrLess(a as String, b as String) as Boolean
    la = Len(a)
    lb = Len(b)
    n = la
    if lb < n then n = lb
    for i = 1 to n
        ca = Asc(Mid(a, i, 1))
        cb = Asc(Mid(b, i, 1))
        if ca <> cb then return ca < cb
    end for
    return la < lb
end function

function MB_CanonicalPath(pathAndQuery as String) as String
    q = Instr(1, pathAndQuery, "?")
    if q = 0 then return pathAndQuery
    path = Left(pathAndQuery, q - 1)
    query = Mid(pathAndQuery, q + 1)
    pairs = []
    for each part in query.Split("&")
        if part <> "" then
            eq = Instr(1, part, "=")
            if eq = 0 then
                pairs.Push({ k: part, v: "" })
            else
                pairs.Push({ k: Left(part, eq - 1), v: Mid(part, eq + 1) })
            end if
        end if
    end for
    if pairs.Count() = 0 then return path

    ' Stable insertion sort by key.
    for i = 1 to pairs.Count() - 1
        current = pairs[i]
        j = i - 1
        while j >= 0 and MB_StrLess(current.k, pairs[j].k)
            pairs[j + 1] = pairs[j]
            j = j - 1
        end while
        pairs[j + 1] = current
    end for

    parts = []
    for each p in pairs
        parts.Push(p.k + "=" + p.v)
    end for
    return path + "?" + parts.Join("&")
end function

' ---------------------------------------------------------------------------
' Persistent client identity (one fake handset per Roku install)
' ---------------------------------------------------------------------------

function MB_Registry() as Object
    return CreateObject("roRegistrySection", "moviebox")
end function

function MB_Identity() as Object
    reg = MB_Registry()
    if reg.Exists("identity") then
        saved = ParseJson(reg.Read("identity"))
        if saved <> invalid and saved.ua <> invalid then return saved
    end if

    androids = [
        ["10", "QP1A.191005.007.A3"]
        ["11", "RP1A.200720.011"]
        ["12", "S1B.220414.015"]
        ["13", "TQ2A.230405.003"]
    ]
    models = ["23078RKD5C", "2201117TY", "2201117TG", "22101316G", "21121210G", "M2012K11AG"]
    codes = ["50020117", "50020118", "50020119", "50020120", "50020121"]
    zones = ["America/New_York", "Europe/London", "Asia/Kolkata", "Asia/Tokyo"]

    android = androids[Rnd(androids.Count()) - 1]
    model = models[Rnd(models.Count()) - 1]
    code = codes[Rnd(codes.Count()) - 1]
    zone = zones[Rnd(zones.Count()) - 1]
    gaid = MB_RandomHex(8) + "-" + MB_RandomHex(4) + "-" + MB_RandomHex(4) + "-" + MB_RandomHex(4) + "-" + MB_RandomHex(12)

    ua = "com.community.oneroom/" + code + " (Linux; U; Android " + android[0] + "; en_US; " + model + "; Build/" + android[1] + "; Cronet/135.0.7012.3)"

    q = Chr(34)
    info = "{"
    info = info + q + "package_name" + q + ":" + q + "com.community.oneroom" + q + ","
    info = info + q + "version_name" + q + ":" + q + "4.0.01.0813.03" + q + ","
    info = info + q + "version_code" + q + ":" + code + ","
    info = info + q + "os" + q + ":" + q + "android" + q + ","
    info = info + q + "os_version" + q + ":" + q + android[0] + q + ","
    info = info + q + "install_ch" + q + ":" + q + "ps" + q + ","
    info = info + q + "device_id" + q + ":" + q + MB_RandomHex(32) + q + ","
    info = info + q + "install_store" + q + ":" + q + "ps" + q + ","
    info = info + q + "gaid" + q + ":" + q + gaid + q + ","
    info = info + q + "brand" + q + ":" + q + "Redmi" + q + ","
    info = info + q + "model" + q + ":" + q + model + q + ","
    info = info + q + "system_language" + q + ":" + q + "en" + q + ","
    info = info + q + "net" + q + ":" + q + "NETWORK_WIFI" + q + ","
    info = info + q + "region" + q + ":" + q + "US" + q + ","
    info = info + q + "timezone" + q + ":" + q + zone + q + ","
    info = info + q + "sp_code" + q + ":" + q + "40401" + q + ","
    info = info + q + "X-Play-Mode" + q + ":" + q + "2" + q

    prefixes = ["103.241", "49.36", "117.195", "106.198", "122.162", "157.32", "182.70", "103.58"]
    ip = prefixes[Rnd(prefixes.Count()) - 1] + "." + Rnd(253).ToStr() + "." + Rnd(253).ToStr()

    identity = { ua: ua, info: info, ip: ip }
    reg.Write("identity", FormatJson(identity))
    reg.Flush()
    return identity
end function

' ---------------------------------------------------------------------------
' Signing
' ---------------------------------------------------------------------------

function MB_SignedHeaders(method as String, pathAndQuery as String, body as Dynamic, token as String) as Object
    identity = MB_Identity()
    now = CreateObject("roDateTime")
    ts = now.AsSeconds().ToStr() + MB_Pad3(now.GetMilliseconds())

    accept = "application/json"
    contentType = "application/json"

    bodyLength = ""
    bodyHash = ""
    if body <> invalid then
        bytes = CreateObject("roByteArray")
        bytes.FromAsciiString(body)
        bodyLength = bytes.Count().ToStr()
        bodyHash = MB_Md5Hex(body)
    end if

    lf = Chr(10)
    canonical = UCase(method) + lf + accept + lf + contentType + lf + bodyLength + lf + ts + lf + bodyHash + lf + MB_CanonicalPath(pathAndQuery)
    signature = ts + "|2|" + MB_HmacMd5Base64(canonical)
    clientToken = ts + "," + MB_Md5Hex(MB_Reverse(ts))

    stamp = ts + "000" + (Rnd(1000) - 1).ToStr()
    buildTag = stamp + "." + MB_Md5Hex(MB_CertSha256() + "oneroom" + stamp)
    q = Chr(34)
    clientInfo = identity.info + "," + q + "X-Client-Build" + q + ":" + q + buildTag + q + "}"

    headers = [
        ["User-Agent", identity.ua]
        ["Accept", accept]
        ["Content-Type", contentType]
        ["x-client-token", clientToken]
        ["x-tr-signature", signature]
        ["x-client-build", buildTag]
        ["x-client-info", clientInfo]
        ["x-client-status", "0"]
        ["x-forwarded-for", identity.ip]
    ]
    if token <> "" then headers.Push(["Authorization", "Bearer " + token])
    return headers
end function

' ---------------------------------------------------------------------------
' HTTP
' ---------------------------------------------------------------------------

function MB_HttpRequest(method as String, url as String, body as Dynamic, headers as Object, timeoutMs as Integer) as Object
    port = CreateObject("roMessagePort")
    xfer = CreateObject("roUrlTransfer")
    xfer.SetMessagePort(port)
    xfer.SetUrl(url)
    xfer.SetCertificatesFile("common:/certs/ca-bundle.crt")
    xfer.InitClientCertificates()
    xfer.RetainBodyOnError(true)
    xfer.EnableEncodings(true)
    for each header in headers
        xfer.AddHeader(header[0], header[1])
    end for

    if method = "POST" then
        started = xfer.AsyncPostFromString(body)
    else
        started = xfer.AsyncGetToString()
    end if
    if not started then return { code: -1, body: "" }

    msg = Wait(timeoutMs, port)
    if Type(msg) = "roUrlEvent" then
        return { code: msg.GetResponseCode(), body: msg.GetString() }
    end if
    xfer.AsyncCancel()
    return { code: -2, body: "" }
end function

function MB_RetryStatus(code as Integer) as Boolean
    for each retry in [403, 406, 407, 429, 500, 502, 503, 504]
        if code = retry then return true
    end for
    return code < 0
end function

function MB_RequestHosts(method as String, pathAndQuery as String, body as Dynamic, token as String) as Object
    hosts = MB_Hosts()
    reg = MB_Registry()
    start = 0
    if reg.Exists("hostIdx") then start = reg.Read("hostIdx").ToInt()
    if start < 0 or start >= hosts.Count() then start = 0

    lastCode = 0
    for i = 0 to hosts.Count() - 1
        idx = (start + i) mod hosts.Count()
        headers = MB_SignedHeaders(method, pathAndQuery, body, token)
        res = MB_HttpRequest(method, hosts[idx] + pathAndQuery, body, headers, 12000)
        lastCode = res.code
        MB_Log(method + " " + hosts[idx] + pathAndQuery + " -> " + res.code.ToStr() + " (" + Len(res.body).ToStr() + " bytes)")
        if res.code = 401 then
            return { ok: false, status: 401, error: "Unauthorized" }
        end if
        if not MB_RetryStatus(res.code) and res.code >= 200 and res.code < 300 then
            json = ParseJson(res.body)
            MB_Log("parsed json valid=" + (json <> invalid).ToStr())
            if json <> invalid then
                if idx <> start then
                    reg.Write("hostIdx", idx.ToStr())
                    reg.Flush()
                end if
                data = json
                if Type(json) = "roAssociativeArray" and json.data <> invalid then data = json.data
                return { ok: true, status: res.code, data: data }
            end if
        else if res.code >= 400 and res.code < 500 and not MB_RetryStatus(res.code) then
            ' Client errors are deterministic: another host or a new session will not help.
            return { ok: false, status: res.code, error: "MovieBox rejected the request (HTTP " + res.code.ToStr() + ")" }
        end if
        if res.code = 429 then Sleep(400)
    end for
    return { ok: false, status: lastCode, exhausted: true, error: "MovieBox servers are not responding" }
end function

' ---------------------------------------------------------------------------
' Session
' ---------------------------------------------------------------------------

function MB_JwtExpiry(token as String) as Integer
    parts = token.Split(".")
    if parts.Count() < 2 then return 0
    payload = parts[1].Replace("-", "+").Replace("_", "/")
    while (Len(payload) mod 4) <> 0
        payload = payload + "="
    end while
    bytes = CreateObject("roByteArray")
    bytes.FromBase64String(payload)
    claims = ParseJson(bytes.ToAsciiString())
    if claims = invalid or claims.exp = invalid then return 0
    if Type(claims.exp) = "roString" or Type(claims.exp) = "String" then return claims.exp.ToInt()
    return Int(claims.exp)
end function

function MB_NowSeconds() as Integer
    return CreateObject("roDateTime").AsSeconds()
end function

function MB_EnsureSession() as String
    reg = MB_Registry()
    if reg.Exists("session") then
        saved = ParseJson(reg.Read("session"))
        if saved <> invalid and saved.token <> invalid then
            expiry = MB_Int(saved.exp)
            if saved.token <> "" and (expiry = 0 or MB_NowSeconds() + 60 < expiry) then return saved.token
        end if
    end if

    res = MB_RequestHosts("POST", "/wefeed-mobile-bff/user-api/visitor-login", "{}", "")
    if not res.ok or res.data = invalid or res.data.token = invalid then return ""
    token = res.data.token
    reg.Write("session", FormatJson({ token: token, exp: MB_JwtExpiry(token) }))
    reg.Flush()
    return token
end function

sub MB_ClearSession()
    reg = MB_Registry()
    reg.Delete("session")
    reg.Flush()
end sub

function MB_Request(method as String, pathAndQuery as String, body as Dynamic) as Object
    token = MB_EnsureSession()
    if token = "" then return { ok: false, error: "Could not connect to MovieBox" }
    res = MB_RequestHosts(method, pathAndQuery, body, token)
    if not res.ok and (res.status = 401 or res.exhausted = true) then
        MB_ClearSession()
        token = MB_EnsureSession()
        if token <> "" then res = MB_RequestHosts(method, pathAndQuery, body, token)
    end if
    return res
end function

function MB_Get(pathAndQuery as String) as Object
    return MB_Request("GET", pathAndQuery, invalid)
end function

function MB_Post(pathAndQuery as String, body as String) as Object
    return MB_Request("POST", pathAndQuery, body)
end function

' ---------------------------------------------------------------------------
' Disk cache for read-only catalogue calls
'
' Only payloads that are safe to reuse go through here: the homepage, search
' results and title details. Signed play-info is never cached because the CDN
' URLs it returns expire.
' ---------------------------------------------------------------------------

function MB_CacheKey(text as String) as String
    out = ""
    lower = LCase(text)
    for i = 1 to Len(lower)
        ch = Mid(lower, i, 1)
        code = Asc(ch)
        if (code >= 97 and code <= 122) or (code >= 48 and code <= 57) then
            out = out + ch
        else
            out = out + "-"
        end if
    end for
    if Len(out) > 90 then out = Right(out, 90)
    return out
end function

function MB_CacheRead(key as String, ttlSeconds as Integer) as Dynamic
    path = "tmp:/cache/" + key + ".json"
    fs = CreateObject("roFileSystem")
    if not fs.Exists(path) then return invalid
    text = ReadAsciiFile(path)
    if text = "" then return invalid
    entry = ParseJson(text)
    if entry = invalid then return invalid
    ts = MB_Int(entry.ts)
    if ts <= 0 then return invalid
    if MB_NowSeconds() - ts >= ttlSeconds then return invalid
    return entry.data
end function

sub MB_CacheWrite(key as String, data as Dynamic)
    ' tmp: is small and shared with the downloaded subtitles, so keep the cache
    ' bounded and skip payloads that would dominate it.
    text = FormatJson({ ts: MB_NowSeconds(), data: data })
    if Len(text) > 400000 then return
    fs = CreateObject("roFileSystem")
    if not fs.Exists("tmp:/cache") then fs.CreateDirectory("tmp:/cache")
    entries = fs.ListDir("tmp:/cache")
    if entries <> invalid and entries.Count() >= 60 then
        for each name in entries
            fs.Delete("tmp:/cache/" + name)
        end for
    end if
    WriteAsciiFile("tmp:/cache/" + key + ".json", text)
end sub

function MB_GetCached(pathAndQuery as String, ttlSeconds as Integer) as Object
    key = MB_CacheKey("g" + pathAndQuery)
    data = MB_CacheRead(key, ttlSeconds)
    if data <> invalid then return { ok: true, data: data, cached: true }
    res = MB_Get(pathAndQuery)
    if res.ok then MB_CacheWrite(key, res.data)
    return res
end function

function MB_PostCached(pathAndQuery as String, body as String, ttlSeconds as Integer) as Object
    key = MB_CacheKey("p" + pathAndQuery + body)
    data = MB_CacheRead(key, ttlSeconds)
    if data <> invalid then return { ok: true, data: data, cached: true }
    res = MB_Post(pathAndQuery, body)
    if res.ok then MB_CacheWrite(key, res.data)
    return res
end function

' ---------------------------------------------------------------------------
' Normalisation helpers
' ---------------------------------------------------------------------------

function MB_Str(value as Dynamic) as String
    if value = invalid then return ""
    t = Type(value)
    if t = "roString" or t = "String" then return value
    if t = "roInt" or t = "Integer" or t = "roLongInteger" or t = "LongInteger" then return value.ToStr()
    if t = "roFloat" or t = "Float" or t = "roDouble" or t = "Double" then return Str(value).Trim()
    if t = "roBoolean" or t = "Boolean" then
        if value then return "true"
        return "false"
    end if
    return ""
end function

function MB_Int(value as Dynamic) as Integer
    if value = invalid then return 0
    t = Type(value)
    if t = "roString" or t = "String" then return value.ToInt()
    if t = "roInt" or t = "Integer" then return value
    if t = "roFloat" or t = "Float" or t = "roDouble" or t = "Double" or t = "roLongInteger" or t = "LongInteger" then return Int(value)
    return 0
end function

function MB_Html(text as String) as String
    return text.Replace("&#39;", "'").Replace("&quot;", Chr(34)).Replace("&amp;", "&").Replace("&lt;", "<").Replace("&gt;", ">")
end function

' Splits "Inception [Hindi]" into title "Inception" and tag "Hindi".
function MB_SplitTitle(raw as String) as Object
    title = MB_Html(raw.Trim())
    tag = ""
    open = Instr(1, title, "[")
    if open > 1 then
        close = Instr(open, title, "]")
        if close > open then tag = Mid(title, open + 1, close - open - 1)
        title = Left(title, open - 1).Trim()
    end if
    return { title: title, tag: tag }
end function

function MB_Year(value as Dynamic) as String
    s = MB_Str(value)
    if Len(s) >= 4 then return Left(s, 4)
    return s
end function

' Builds a resized JPEG URL from the image CDN (cuts multi-MB covers down to a few KB).
function MB_Image(url as Dynamic, width as Integer) as String
    u = MB_Str(url)
    if u = "" then return ""
    if Instr(1, u, "aoneroom.com") = 0 then return u
    sep = "?"
    if Instr(1, u, "?") > 0 then sep = "&"
    return u + sep + "x-oss-process=image/resize,w_" + width.ToStr() + "/format,jpg"
end function

function MB_CoverUrl(subject as Object) as String
    if subject.cover <> invalid and Type(subject.cover) = "roAssociativeArray" then return MB_Str(subject.cover.url)
    return MB_Str(subject.coverUrl)
end function

function MB_StillUrl(subject as Object) as String
    if subject.stills <> invalid and Type(subject.stills) = "roAssociativeArray" then return MB_Str(subject.stills.url)
    return ""
end function

' Content the channel hides: Hindi dubs/versions and Indian-origin titles.
function MB_IsHiddenLanguage(text as String) as Boolean
    if not HideHindiActive() then return false
    return Instr(1, LCase(text), "hindi") > 0
end function

' Indian-origin titles are dropped outright; Hindi dubs of foreign titles are kept
' but shown as the original (details/playback redirect to the original version).
function MB_IsHiddenSubject(country as String) as Boolean
    if not HideHindiActive() then return false
    return LCase(country.Trim()) = "india"
end function

' Row titles from the regional feed that only carry Indian content.
function MB_IsHiddenRow(title as String) as Boolean
    if not HideHindiActive() then return false
    ' India-curated banners are named like "IN Banner" or "Banner_movie tab_IN".
    for each token in title.Replace("_", " ").Split(" ")
        if token = "IN" then return true
    end for
    t = LCase(title)
    for each word in ["hindi", "india", "bollywood", "desi", "punjabi", "tamil", "telugu"]
        if Instr(1, t, word) > 0 then return true
    end for
    return false
end function

' Friendly names for raw dub codes such as "esla dub" or "ptbr dub".
function MB_DubName(name as String, code as String) as String
    names = { esla: "Spanish (Latin America)", es: "Spanish", ptbr: "Portuguese (Brazil)", pt: "Portuguese", fr: "French", de: "German", it: "Italian", ja: "Japanese", ko: "Korean", zh: "Chinese", ar: "Arabic", ru: "Russian", tr: "Turkish", id: "Indonesian", ta: "Tamil", te: "Telugu" }
    c = LCase(code)
    if LCase(name) = c + " dub" and names[c] <> invalid then return names[c]
    if names[c] <> invalid and Right(LCase(name), 4) = " dub" then return names[c]
    return name
end function

function MB_NormalizeSubject(subject as Dynamic) as Dynamic
    if subject = invalid or Type(subject) <> "roAssociativeArray" then return invalid
    id = MB_Str(subject.subjectId)
    if id = "" then id = MB_Str(subject.id)
    if id = "" then return invalid
    stype = MB_Int(subject.subjectType)
    if stype <> 1 and stype <> 2 then return invalid
    if subject.hasResource <> invalid and subject.hasResource = false then return invalid

    names = MB_SplitTitle(MB_Str(subject.title))
    if names.title = "" then return invalid
    if MB_IsHiddenSubject(MB_Str(subject.countryName)) then return invalid
    if MB_IsHiddenLanguage(names.tag) then names.tag = ""
    cover = MB_CoverUrl(subject)
    still = MB_StillUrl(subject)
    if still = "" then still = cover

    rating = MB_Str(subject.imdbRatingValue)
    if rating = "" then rating = MB_Str(subject.imdbRate)
    if rating = "0" or rating = "0.0" then rating = ""

    return {
        id: id
        stype: stype
        title: names.title
        tag: names.tag
        year: MB_Year(subject.releaseDate)
        genre: MB_Str(subject.genre).Replace(",", ", ").Replace(",  ", ", ")
        rating: rating
        duration: MB_Str(subject.duration)
        country: MB_Str(subject.countryName)
        description: MB_Html(MB_Str(subject.description))
        poster: MB_Image(cover, 320)
        backdrop: MB_Image(still, 1280)
        wide: ""
    }
end function

' Creates a ContentNode for a normalised subject.
function MB_ItemNode(item as Object) as Object
    node = CreateObject("roSGNode", "ContentNode")
    node.title = item.title
    node.description = item.description
    node.HDPosterUrl = item.poster
    node.addFields({
        sid: item.id
        stype: item.stype
        tag: item.tag
        year: item.year
        genre: item.genre
        rating: item.rating
        durationText: item.duration
        country: item.country
        backdrop: item.backdrop
        wide: item.wide
        progress: 0.0
        se: 0
        ep: 0
        resumeAt: 0
        subtitle: ""
    })
    return node
end function
