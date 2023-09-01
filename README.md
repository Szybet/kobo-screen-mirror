# kobo-screen-mirror
*Death simple* tool to mirror your kobo screen to PC if you are tired of clicking
![kobo-screen-mirrorr](https://github.com/Szybet/kobo-screen-mirror/assets/53944559/225a464e-9c7b-4050-8aa1-5e214d6e02f3)

This is a tool for developers, I don't see any reasons why anyone else would use it. The target audience will figure out how to use it... ;)

TODO: If anyone is interested in improving this:
- Create fbink-rs and use native library calls, I made it use fbgrab and didn't cared to change it because it works well enough
- Figure out how to get mouse input clicks of an image in egui, it would enable adding some more widgets like showing fps, a force refresh button etc.

At least some notes:
- Needed, sister project: https://github.com/Kobo-InkBox/touch_emulate
- Sunxi SOC are stupid and won't work with this tool because they have per app buffer, blame the chinese? or kernel hacks?...
- use USBNET
