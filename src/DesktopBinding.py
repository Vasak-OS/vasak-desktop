import json
import os
from Vasak.system.VSKIconManager import VSKIconManager
from PyQt6.QtCore import pyqtSlot, QObject

class DesktopBinding(QObject):
  def __init__(self, window):
    super().__init__()
    self.window = window
    self.iconsManager = VSKIconManager()
  
  @pyqtSlot(str, result=str)
  def getGlobalIcon(self, iconName):
      return self.iconsManager.get_icon(iconName)

  @pyqtSlot(result=str)
  def getHome(self):
      home_path = os.path.expanduser("~")

      if not os.path.isabs(home_path):
        home_path = os.path.join("/", home_path)

      return home_path