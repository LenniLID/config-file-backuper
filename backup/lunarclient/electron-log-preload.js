
      try {
        (function $({contextBridge:V,ipcRenderer:J}){if(!J)return;J.on("__ELECTRON_LOG_IPC__",(ee,te)=>{window.postMessage({cmd:"message",...te})}),J.invoke("__ELECTRON_LOG__",{cmd:"getOptions"}).catch(ee=>console.error(new Error(`electron-log isn't initialized in the main process. Please call log.initialize() before. ${ee.message}`)));const K={sendToMain(ee){try{J.send("__ELECTRON_LOG__",ee)}catch(te){console.error("electronLog.sendToMain ",te,"data:",ee),J.send("__ELECTRON_LOG__",{cmd:"errorHandler",error:{message:te==null?void 0:te.message,stack:te==null?void 0:te.stack},errorName:"sendToMain"})}},log(...ee){K.sendToMain({data:ee,level:"info"})}};for(const ee of["error","warn","info","verbose","debug","silly"])K[ee]=(...te)=>K.sendToMain({data:te,level:ee});if(V&&process.contextIsolated)try{V.exposeInMainWorld("__electronLog",K)}catch{}typeof window=="object"?window.__electronLog=K:__electronLog=K})(require('electron'));
      } catch(e) {
        console.error(e);
      }
    