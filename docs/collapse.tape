Output docs/collapse.gif

Set FontSize 22
Set Width 700
Set Height 360
Set Padding 15
Set Framerate 20

# Filled with Space, half of it off camera, so the recording opens on a pile that is
# about to give way. It ends on the empty floor: the next round starts 0.6s later and
# would drop a whole new pile into the clip.
Hide
Type "COLORTERM=truecolor ./target/release/sunayama"
Enter
Sleep 1s
Space@40ms 30
Show
Space@40ms 25
Sleep 1700ms
Hide
Ctrl+C
