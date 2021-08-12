var option = {
  key : "Escape",
  active : function() {
    nw.App.quit(); 
  },
  failed : function(msg) {
    // :(, fail to register the |key| or couldn't parse the |key|.
    console.log(msg);
  }
};

// Create a shortcut with |option|.
var shortcut = new nw.Shortcut(option);

nw.App.registerGlobalHotKey(shortcut);
