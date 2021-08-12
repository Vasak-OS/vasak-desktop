// Set windows Properties
win.setShowInTaskbar(false);
win.resizeTo(Math.round(screen.width), Math.round(screen.height));
win.setResizable(false);

exec('python', ["scripts/setDesktop.py"]);

const video = document.getElementById('video-background');
video.src = `file://${homePath}.config/lynx/background/tropical-sunset.mp4`;
