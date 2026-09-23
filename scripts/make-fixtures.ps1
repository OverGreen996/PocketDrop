$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
$out = Join-Path $root 'fixtures'
New-Item -ItemType Directory -Force $out, "$root\src-tauri\icons" | Out-Null
foreach ($mode in @('dark','light','multicolor')) {
  $bitmap = New-Object System.Drawing.Bitmap 1920,1080
  $g = [System.Drawing.Graphics]::FromImage($bitmap)
  $colors = switch ($mode) {
    dark { @('#0c1829','#273f54','#173b38') }
    light { @('#fff6e7','#d9f6f0','#dbe8ff') }
    multicolor { @('#c95656','#49bba7','#6271dc') }
  }
  for ($i = 0; $i -lt 3; $i++) {
    $rect = New-Object System.Drawing.Rectangle ($i * 640),0,640,1080
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect,([System.Drawing.ColorTranslator]::FromHtml($colors[$i])),([System.Drawing.ColorTranslator]::FromHtml($colors[(($i+1)%3)])),90
    $g.FillRectangle($brush,$rect)
    $brush.Dispose()
  }
  $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(100,255,255,255)),2
  for ($x = 0; $x -lt 1920; $x += 24) { $g.DrawLine($pen,$x,250,$x,850) }
  $font = New-Object System.Drawing.Font 'Segoe UI',24
  $g.DrawString("POCKETDROP M0 / $mode / EXTERNAL BACKGROUND",$font,[System.Drawing.Brushes]::White,40,45)
  $bitmap.Save((Join-Path $out "wallpaper-$mode.png"),[System.Drawing.Imaging.ImageFormat]::Png)
  $font.Dispose(); $pen.Dispose(); $g.Dispose(); $bitmap.Dispose()
}
$icon = New-Object System.Drawing.Bitmap 64,64
$g = [System.Drawing.Graphics]::FromImage($icon)
$g.Clear([System.Drawing.Color]::FromArgb(255,32,60,70))
$pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255,173,245,218)),3
$g.DrawRectangle($pen,16,18,32,30)
$g.DrawLine($pen,16,18,32,9); $g.DrawLine($pen,32,9,48,18); $g.DrawLine($pen,32,18,32,48)
$icon.Save("$root\src-tauri\icons\icon.png",[System.Drawing.Imaging.ImageFormat]::Png)
$pngBytes = [System.IO.File]::ReadAllBytes("$root\src-tauri\icons\icon.png")
$stream = [System.IO.File]::Create("$root\src-tauri\icons\icon.ico")
$writer = New-Object System.IO.BinaryWriter $stream
$writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]1)
$writer.Write([byte]64); $writer.Write([byte]64); $writer.Write([byte]0); $writer.Write([byte]0)
$writer.Write([uint16]1); $writer.Write([uint16]32); $writer.Write([uint32]$pngBytes.Length); $writer.Write([uint32]22)
$writer.Write($pngBytes); $writer.Dispose()
$g.Dispose(); $pen.Dispose(); $icon.Dispose()
Set-Content -LiteralPath "$out\sample.txt" -Value 'PocketDrop M0 local drag test only.' -Encoding UTF8
Set-Content -LiteralPath "$out\sample.stl" -Value "solid test`nendsolid test" -Encoding ASCII
New-Item -ItemType Directory -Force "$out\folder-rejection" | Out-Null
Set-Content -LiteralPath "$out\folder-rejection\README.txt" -Value 'Drag the parent folder: it must be rejected.'
Write-Output 'Fixtures generated. No Windows wallpaper settings were changed.'
