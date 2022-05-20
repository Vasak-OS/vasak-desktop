// Set windows Properties
win.setShowInTaskbar(false);
win.resizeTo(Math.round(screen.width), Math.round(screen.height));
win.setResizable(false);
win.y = 0;
win.x = 0;
win.setVisibleOnAllWorkspaces(true);

exec('python', ['/usr/share/Lynx/lynx-desktop-service/Setters/setDesktop.py', `${process.pid.toString()}`]);

const video = document.getElementById('video-background');
video.src = `file://${homePath}.background.video`;
