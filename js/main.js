// Set windows Properties
win.setShowInTaskbar(false);
win.resizeTo(Math.round(screen.width)-1, Math.round(screen.height)-1);
win.setResizable(false);

exec('python', ["/usr/share/Lynx/lynx-desktop-service/Setters/setDesktop.py"]);

const video = document.getElementById('video-background');
video.src = `file://${homePath}.config/lynx/background/tropical-sunset.mp4`;
