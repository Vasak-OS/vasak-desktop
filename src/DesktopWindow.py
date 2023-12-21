from Vasak.VSKWindow import VSKWindow
from PyQt6.QtWidgets import QApplication
from PyQt6.QtCore import Qt
from src.DesktopBinding import DesktopBinding

class DesktopWindow(VSKWindow):
    def __init__(self):
        super().__init__()
        self.shareObject = DesktopBinding(self)
        self.channel.registerObject("vsk", self.shareObject)
        self.move_to_screen() # Mover la ventana a una pantalla específica
        self.set_as_desktop() # Hacer que la ventana se comporte como un dock
        self.load_html("ui/dist/index.html") # Cargar un HTML en el WebView
        
    # Mover la ventana a una pantalla específica
    def move_to_screen(self):
        self.setGeometry(0, 0, QApplication.primaryScreen().availableGeometry().width(), QApplication.primaryScreen().availableGeometry().height())
    
    def set_as_desktop(self):
        self.setAttribute(Qt.WidgetAttribute.WA_X11NetWmWindowTypeDesktop, True)  # Seteo tipo desktop x11
        self.setAttribute(Qt.WidgetAttribute.WA_AlwaysStackOnTop, False)
        self.setWindowFlags(
            self.windowFlags() | Qt.WindowType.FramelessWindowHint | Qt.WindowType.Desktop
        )