param(
    [string]$Source = "$PSScriptRoot\..\src\DeepSeekHarnessLauncher\Assets\dsh-logo-transparent.png",
    [string]$Destination = "$PSScriptRoot\..\src\DeepSeekHarnessLauncher\Assets\dsh-tray-icon.ico",
    [string]$Preview = "$PSScriptRoot\..\src\DeepSeekHarnessLauncher\Assets\dsh-tray-icon.png"
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$sizes = @(16, 20, 24, 32, 48, 64, 128, 256)
$sourceBitmap = [System.Drawing.Bitmap]::FromFile((Resolve-Path -LiteralPath $Source))

function Get-VisibleBounds {
    param([System.Drawing.Bitmap]$Bitmap)

    $minX = $Bitmap.Width
    $minY = $Bitmap.Height
    $maxX = -1
    $maxY = -1

    for ($y = 0; $y -lt $Bitmap.Height; $y++) {
        for ($x = 0; $x -lt $Bitmap.Width; $x++) {
            if ($Bitmap.GetPixel($x, $y).A -le 8) {
                continue
            }

            $minX = [Math]::Min($minX, $x)
            $minY = [Math]::Min($minY, $y)
            $maxX = [Math]::Max($maxX, $x)
            $maxY = [Math]::Max($maxY, $y)
        }
    }

    if ($maxX -lt $minX -or $maxY -lt $minY) {
        throw "The source image does not contain visible pixels: $Source"
    }

    return New-Object System.Drawing.Rectangle(
        $minX,
        $minY,
        ($maxX - $minX + 1),
        ($maxY - $minY + 1))
}

function New-TrayBitmap {
    param(
        [int]$Size,
        [System.Drawing.Rectangle]$SourceBounds
    )

    $bitmap = New-Object System.Drawing.Bitmap(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppPArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)

    try {
        $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceOver
        $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.Clear([System.Drawing.Color]::Transparent)

        $badgeMargin = [Math]::Max(0.25, $Size * 0.015)
        $badgeDiameter = $Size - (2 * $badgeMargin)
        $badgeBounds = New-Object System.Drawing.RectangleF(
            $badgeMargin,
            $badgeMargin,
            $badgeDiameter,
            $badgeDiameter)

        $badgeBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
        try {
            $graphics.FillEllipse($badgeBrush, $badgeBounds)
        }
        finally {
            $badgeBrush.Dispose()
        }

        $logoWidth = $Size * 0.88
        $logoHeight = $logoWidth * $SourceBounds.Height / $SourceBounds.Width
        $logoBounds = New-Object System.Drawing.RectangleF(
            (($Size - $logoWidth) / 2),
            (($Size - $logoHeight) / 2),
            $logoWidth,
            $logoHeight)
        $graphics.DrawImage(
            $sourceBitmap,
            $logoBounds,
            $SourceBounds,
            [System.Drawing.GraphicsUnit]::Pixel)

        if ($Size -le 24) {
            for ($y = 0; $y -lt $Size; $y++) {
                for ($x = 0; $x -lt $Size; $x++) {
                    $pixel = $bitmap.GetPixel($x, $y)
                    if ($pixel.A -eq 0) {
                        continue
                    }

                    $brightness = ($pixel.R + $pixel.G + $pixel.B) / 3
                    if ($brightness -lt 160) {
                        $bitmap.SetPixel($x, $y, [System.Drawing.Color]::Black)
                    }
                    else {
                        $bitmap.SetPixel(
                            $x,
                            $y,
                            [System.Drawing.Color]::FromArgb($pixel.A, 255, 255, 255))
                    }
                }
            }
        }

        return $bitmap
    }
    finally {
        $graphics.Dispose()
    }
}

try {
    $sourceBounds = Get-VisibleBounds -Bitmap $sourceBitmap
    $frames = foreach ($size in $sizes) {
        $bitmap = New-TrayBitmap -Size $size -SourceBounds $sourceBounds
        try {
            $stream = New-Object System.IO.MemoryStream
            try {
                $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
                [pscustomobject]@{
                    Size = $size
                    Bytes = $stream.ToArray()
                }
            }
            finally {
                $stream.Dispose()
            }
        }
        finally {
            $bitmap.Dispose()
        }
    }

    $previewBitmap = New-TrayBitmap -Size 512 -SourceBounds $sourceBounds
    try {
        $previewDirectory = Split-Path -Parent $Preview
        New-Item -ItemType Directory -Force -Path $previewDirectory | Out-Null
        $previewBitmap.Save($Preview, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $previewBitmap.Dispose()
    }
}
finally {
    $sourceBitmap.Dispose()
}

$destinationDirectory = Split-Path -Parent $Destination
New-Item -ItemType Directory -Force -Path $destinationDirectory | Out-Null
$fileStream = [System.IO.File]::Create($Destination)
$writer = New-Object System.IO.BinaryWriter($fileStream)

try {
    $writer.Write([UInt16]0)
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]$frames.Count)

    $offset = 6 + (16 * $frames.Count)
    foreach ($frame in $frames) {
        $dimension = if ($frame.Size -eq 256) { [byte]0 } else { [byte]$frame.Size }
        $writer.Write($dimension)
        $writer.Write($dimension)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([UInt16]1)
        $writer.Write([UInt16]32)
        $writer.Write([UInt32]$frame.Bytes.Length)
        $writer.Write([UInt32]$offset)
        $offset += $frame.Bytes.Length
    }

    foreach ($frame in $frames) {
        $writer.Write($frame.Bytes)
    }
}
finally {
    $writer.Dispose()
    $fileStream.Dispose()
}

Write-Output "Generated $Destination and $Preview"
