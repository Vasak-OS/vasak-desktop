// Set windows Properties
win.setShowInTaskbar(false);
win.setResizable(false);
win.y = 0;
win.x = 0;
win.setVisibleOnAllWorkspaces(true);

exec('python', ['/usr/share/Lynx/lynx-desktop-service/Setters/setDesktop.py', `${process.pid.toString()}`]);

const video = document.getElementById('video-background');
video.src = `file://${homePath}.background.video`;
