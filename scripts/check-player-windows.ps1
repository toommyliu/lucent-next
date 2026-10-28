param([string]$Shots = (Join-Path ([IO.Path]::GetTempPath()) ("lucent-shots-" + [guid]::NewGuid().ToString('N'))))
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot)
New-Item -ItemType Directory -Force $Shots | Out-Null
$logs = Join-Path ([IO.Path]::GetTempPath()) ("lucent-check-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force $logs | Out-Null

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

public static class LucentCheck {
    [StructLayout(LayoutKind.Sequential)] struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] struct POINT { public int X, Y; }
    [DllImport("user32.dll")] static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr hwnd, ref POINT point);
    [DllImport("user32.dll")] static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int cx, int cy, uint flags);

    const uint SWP_NOSIZE = 0x1, SWP_NOMOVE = 0x2, SWP_NOZORDER = 0x4, SWP_NOACTIVATE = 0x10;

    public static void DpiAware() { SetProcessDpiAwarenessContext(new IntPtr(-4)); }

    public static double Scale(IntPtr hwnd) { return GetDpiForWindow(hwnd) / 96.0; }

    public static Rectangle Client(IntPtr hwnd) {
        RECT rect;
        GetClientRect(hwnd, out rect);
        POINT origin = new POINT();
        ClientToScreen(hwnd, ref origin);
        return new Rectangle(origin.X, origin.Y, rect.Right - rect.Left, rect.Bottom - rect.Top);
    }

    public static void Raise(IntPtr hwnd) {
        SetWindowPos(hwnd, new IntPtr(-1), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }

    public static void Place(IntPtr hwnd, Rectangle bounds) {
        SetWindowPos(hwnd, IntPtr.Zero, bounds.X, bounds.Y, bounds.Width, bounds.Height, SWP_NOZORDER | SWP_NOACTIVATE);
    }

    public static Bitmap Capture(Rectangle area) {
        Bitmap shot = new Bitmap(area.Width, area.Height, PixelFormat.Format32bppArgb);
        using (Graphics g = Graphics.FromImage(shot)) { g.CopyFromScreen(area.Location, Point.Empty, area.Size); }
        return shot;
    }

    public static int[] Count(Bitmap shot, int top) {
        BitmapData data = shot.LockBits(new Rectangle(0, 0, shot.Width, shot.Height), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
        int stride = data.Stride / 4;
        int[] pixels = new int[stride * shot.Height];
        Marshal.Copy(data.Scan0, pixels, 0, pixels.Length);
        shot.UnlockBits(data);
        int red = 0, uncovered = 0, total = 0;
        for (int y = top; y < shot.Height; y++) {
            for (int x = 0; x < shot.Width; x++) {
                int p = pixels[y * stride + x];
                int r = (p >> 16) & 255, g = (p >> 8) & 255, b = p & 255;
                total++;
                if (r == 255 && g == 0 && b == 0) red++;
                if (r > 200 && g < 60 && b > 200) uncovered++;
            }
        }
        return new int[] { red, uncovered, total };
    }
}
'@
[LucentCheck]::DpiAware()

function Fail([string]$message) {
    throw "$message Logs: $logs"
}

function Pump([double]$seconds) {
    $until = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $until) {
        [Windows.Forms.Application]::DoEvents()
        Start-Sleep -Milliseconds 100
    }
}

function Wait-Until([scriptblock]$condition, [double]$seconds, [string]$message) {
    $until = (Get-Date).AddSeconds($seconds)
    while (-not (& $condition)) {
        if ($app.HasExited) { Fail "Lucent exited." }
        if ((Get-Date) -gt $until) { Fail $message }
        Pump 1
    }
}

function Last-Frame {
    Get-Content $log -ErrorAction SilentlyContinue | Select-String -Pattern 'fps=(\d+) viewport=(\d+)x(\d+)@\S+' | Select-Object -Last 1
}

function Check-Shot([string]$name) {
    $client = [LucentCheck]::Client($window)
    $top = [int][Math]::Round(30 * [LucentCheck]::Scale($window))
    $path = Join-Path $Shots "$name.png"
    $shot = [LucentCheck]::Capture($client)
    $shot.Save($path, [Drawing.Imaging.ImageFormat]::Png)
    $counts = [LucentCheck]::Count($shot, $top)
    $shot.Dispose()
    $red, $uncovered, $total = $counts
    if ($red -lt 14400) { Fail "The overlay box isn't above the game in ${name}: $red red pixels." }
    if ($uncovered -gt $total / 100) { Fail "The game doesn't fill the game area in ${name}: $uncovered of $total pixels show the backdrop." }
    $frame = (Last-Frame).Matches[0].Value
    "${name}: $frame, client $($client.Width)x$($client.Height), $red red pixels, $uncovered backdrop pixels, $path"
}

cargo build -p lucent-app --features overlay-check
if ($LASTEXITCODE -ne 0) { Fail "The build failed." }
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
$bin = Resolve-Path (Join-Path $target 'debug\lucent.exe')
$log = Join-Path $logs 'lucent.log'

$screen = [Windows.Forms.Screen]::PrimaryScreen
$backdrop = New-Object Windows.Forms.Form
$backdrop.FormBorderStyle = 'None'
$backdrop.ShowInTaskbar = $false
$backdrop.StartPosition = 'Manual'
$backdrop.Bounds = $screen.Bounds
$backdrop.BackColor = [Drawing.Color]::FromArgb(255, 0, 255)
$backdrop.TopMost = $true
$backdrop.Show()
$app = $null
try {
    $env:RUST_LOG = 'lucent_host::player=debug'
    $app = Start-Process $bin -PassThru -NoNewWindow -RedirectStandardOutput $log -RedirectStandardError (Join-Path $logs 'stderr.log')
    Wait-Until { $app.Refresh(); $app.MainWindowHandle -ne 0 } 30 "No game window within 30 seconds."
    $window = $app.MainWindowHandle
    [LucentCheck]::Raise($window)
    Wait-Until { $frame = Last-Frame; $frame -and [int]$frame.Matches[0].Groups[1].Value -ge 2 } 60 "No frames shown within 60 seconds."
    Pump 15
    Check-Shot 'windows'

    $before = (Last-Frame).Matches[0].Groups
    $work = $screen.WorkingArea
    $size = New-Object Drawing.Size 1280, 960
    [LucentCheck]::Place($window, (New-Object Drawing.Rectangle ($work.X + ($work.Width - $size.Width) / 2), ($work.Y + ($work.Height - $size.Height) / 2), $size.Width, $size.Height))
    Wait-Until {
        $frame = Last-Frame
        $frame -and $frame.Matches[0].Groups[2].Value -ne $before[2].Value -and [int]$frame.Matches[0].Groups[1].Value -ge 2
    } 15 "The player didn't follow the window to its new size within 15 seconds."
    Pump 3
    Check-Shot 'windows-resized'

    $closing = Get-Date
    [void]$app.CloseMainWindow()
    if (-not $app.WaitForExit(10000)) { Fail "Lucent didn't exit within 10 seconds of closing its window." }
    "closed in $([int]((Get-Date) - $closing).TotalMilliseconds) ms"
} finally {
    if ($app -and -not $app.HasExited) { $app.Kill() }
    $backdrop.Close()
}
