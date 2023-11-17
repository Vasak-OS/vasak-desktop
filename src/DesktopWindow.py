from Vasak.VSKWindow import VSKWindow
from PyQt6.QtWidgets import QApplication
from src.DesktopBinding import DesktopBinding

class DesktopWindow(VSKWindow):
    def __init__(self):
        super().__init__()
        self.shareObject = DesktopBinding(self)
        self.channel.registerObject("vsk", self.shareObject)
        self.move_to_screen() # Mover la ventana a una pantalla específica
        self.set_as_dock() # Hacer que la ventana se comporte como un dock
        self.load_html("ui/dist/index.html") # Cargar un HTML en el WebView
        
    
    # Mover la ventana a una pantalla específica
    def move_to_screen(self):
        self.setGeometry(0, 0, QApplication.primaryScreen().availableGeometry().width(), QApplication.primaryScreen().availableGeometry().height())