' MovieBox for Roku - application entry point.
sub Main(args as Dynamic)
    screen = CreateObject("roSGScreen")
    port = CreateObject("roMessagePort")
    screen.SetMessagePort(port)

    scene = screen.CreateScene("MainScene")
    screen.Show()

    if args <> invalid and args.contentId <> invalid then
        scene.launchArgs = { contentId: args.contentId, mediaType: args.mediaType }
    end if

    scene.ObserveField("exitApp", port)
    input = CreateObject("roInput")
    input.SetMessagePort(port)

    while true
        msg = Wait(0, port)
        msgType = Type(msg)
        if msgType = "roSGScreenEvent" then
            if msg.IsScreenClosed() then return
        else if msgType = "roSGNodeEvent" then
            if msg.GetField() = "exitApp" and msg.GetData() = true then
                screen.Close()
                return
            end if
        else if msgType = "roInputEvent" then
            info = msg.GetInfo()
            if info <> invalid and info.contentId <> invalid then
                scene.launchArgs = { contentId: info.contentId, mediaType: info.mediaType }
            end if
        end if
    end while
end sub
