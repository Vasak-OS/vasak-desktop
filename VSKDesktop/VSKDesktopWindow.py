from Vasak.VSKWindow import VSKWindow

class VSKDesktopWindow(VSKWindow):
    def __init__(self, screen_num=0):
        super().__init__(screen_num)
        #self.set_title("vasak-desktop")
        self.set_as_desktop()
        self.showFullScreen()
        self.load_html("index.html")