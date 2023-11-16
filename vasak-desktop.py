import sys
import signal
from VSKDesktop.VSKDesktopWindow import VSKDesktopWindow
from PyQt6.QtWidgets import QApplication


app = QApplication(sys.argv)

if __name__ == "__main__":
    window = VSKDesktopWindow()
    window.show()
    signal.signal(signal.SIGINT, signal.SIG_DFL)  # Habilitar Ctrl+C
    sys.exit(app.exec())
