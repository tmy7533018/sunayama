"""Cuts a recorded clip down to the frames worth showing.

The opening pours a random amount, so nothing lands at the same timestamp twice
and the tapes cannot window the action on their own. Record generously, look at
the clip, then cut. Frames are 20 to the second.

Usage: python3 docs/trim.py docs/opening.gif 0 46
"""

import subprocess
import sys


def main(gif, first, last):
    out = gif.removesuffix(".gif") + ".trimmed.gif"
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", gif, "-vf",
         f"select='between(n\\,{first}\\,{last})',setpts=PTS-STARTPTS,"
         "split[a][b];[a]palettegen[p];[b][p]paletteuse",
         "-loop", "0", out], check=True)
    subprocess.run(["mv", out, gif], check=True)
    print(f"{gif}: kept {first}-{last} ({(last - first + 1) / 20:.1f}s)")


main(sys.argv[1], int(sys.argv[2]), int(sys.argv[3]))
