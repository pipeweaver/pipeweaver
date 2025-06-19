import {store} from '@/app/store.js'

export class Websocket {
  #connection_promise = []
  #disconnect_callback = undefined
  #message_queue = []
  #websocket = undefined
  #command_index = 0

  connect() {
    this.#websocket = new WebSocket(getWebsocketAddress())

    let self = this
    self.#websocket.addEventListener('message', function (event) {
      // A message can be one of two things, either a DaemonStatus, or an error..
      let json = JSON.parse(event.data)

      let message_id = json.id
      let message_data = json.data
      if (message_data['Status'] !== undefined) {
        self.#fulfill_promise(message_id, message_data, true)
      } else if (message_data['Patch'] !== undefined) {
        // Nothing ever requests patch data, so we can ignore this.
        store.patchData(message_data)
      } else if (message_data === 'Ok' || message_data['Pipewire'] !== undefined) {
        self.#fulfill_promise(message_id, message_data, true)
      } else {
        self.#fulfill_promise(message_id, message_data, false)
        console.log('Received Error from Websocket: ' + event.data)
      }
    })

    self.#websocket.addEventListener('open', function () {
      console.log('OPEN')
      if (self.#connection_promise[0] !== undefined) {
        self.#connection_promise[0]()
      }
      self.#connection_promise = []
    })

    self.#websocket.addEventListener('close', function () {
      if (self.#connection_promise[1] !== undefined) {
        self.#connection_promise[1]()
      }
      self.#connection_promise = []

      if (self.#disconnect_callback !== undefined) {
        self.#disconnect_callback()
        self.#disconnect_callback = undefined
      }

      self.#websocket.close()
    })

    self.#websocket.addEventListener('error', function () {
      if (self.#connection_promise[1] !== undefined) {
        self.#connection_promise[1]()
      }
      self.#connection_promise = []

      if (self.#disconnect_callback !== undefined) {
        self.#disconnect_callback()
        self.#disconnect_callback = undefined
      }
      self.#websocket.close()
    })

    return new Promise((resolve, reject) => {
      self.#connection_promise[0] = resolve
      self.#connection_promise[1] = reject
    })
  }

  is_connected() {
    return (this.#websocket.readyState === WebSocket.OPEN);
  }

  on_disconnect(func) {
    this.#disconnect_callback = func
  }

  get_status() {
    return this.#sendRequest('GetStatus')
  }

  open_path(type) {
    let request = {
      OpenPath: type
    }

    return this.send_daemon_command(request)
  }

  send_daemon_command(command) {
    let request = {
      Daemon: command
    }
    return this.#sendRequest(request)
  }

  send_command(command) {
    let request = {
      Pipewire: command
    }
    return this.#sendRequest(request)
  }

  #sendRequest(request) {
    let id = this.#command_index++

    // Wrap this request with an ID
    let final_request = {
      id: id,
      data: request
    }

    this.#websocket.send(JSON.stringify(final_request))

    // Create and return a response promise...
    let self = this
    return new Promise((resolve, reject) => {
      self.#message_queue[id] = []
      self.#message_queue[id][0] = resolve
      self.#message_queue[id][1] = reject
    })
  }

  #fulfill_promise(id, data, is_success) {
    if (this.#message_queue[id] !== undefined) {
      this.#message_queue[id][is_success ? 0 : 1](data)
      delete this.#message_queue[id]
    }
  }
}

export const websocket = new Websocket()

/*
  This is a comparatively dumb websocket, meter data is simply 'Push on Receive', so we just
  need a basic connect / disconnect and handle behaviour
 */
export class WebsocketMeter {
  #index = 0;
  #websocket = undefined;
  #callbacks = [];

  connect() {
    this.#websocket = new WebSocket(getWebsocketAddress() + "/meter");

    let self = this
    self.#websocket.addEventListener('message', function (event) {
      let json = JSON.parse(event.data);
      for (const callback of self.#callbacks) {
        if (callback.node === json.id) {
          callback.executor(json.percent);
        }
      }
    });

    self.#websocket.addEventListener('close', function () {
      self.#websocket.close()
    })

    self.#websocket.addEventListener('error', function () {
      self.#websocket.close()
    })
  }

  register_callback(node, executor) {
    let index = this.#index++;
    this.#callbacks.push({
      index: index, node: node, executor: executor,
    });
    return index;
  }

  unregister_callback(id) {
    this.#callbacks = this.#callbacks.filter(function (item) {
      return item.index !== id
    });
  }

  disconnect() {
    this.#websocket.close();
  }
}

let window_visible = true;
export const websocket_meter = new WebsocketMeter()
document.addEventListener("visibilitychange", () => {
  window_visible = !document.hidden;
  if (window_visible) {
    if (websocket.is_connected()) {
      websocket_meter.connect();
    }
  } else {
    websocket_meter.disconnect();
  }
});

export function runWebsocket() {
  console.log('Connecting..')
  // Let's attempt to connect the websocket...
  websocket
    .connect()
    .then(() => {
      // We got a connection, try fetching the status...
      websocket.get_status().then((data) => {
        store.socketConnected(data)
        if (window_visible) {
          websocket_meter.connect();
        }

        websocket.on_disconnect(() => {
          websocket_meter.disconnect();
          store.socketDisconnected()
          setTimeout(runWebsocket, 1000)
        })
      })
    })
    .catch(() => {
      // Wait 1 second, then try again..
      setTimeout(runWebsocket, 1000)
    })
}

/*
 * This function simply sends a command via HTTP and returns a promise of a response.
 *
 * The GoXLR Daemon simply handles a DaemonRequest and returns a DaemonResponse, it doesn't do anything special for
 * errors, so we'll handle fulfill or reject here based on what comes back.
 */
export function sendHttpCommand(serial, command) {
  let request = {
    Command: [serial, command]
  }
  return executeHttpRequest(request)
}

function executeHttpRequest(request) {
  let cmd_resolve, cmd_reject

  fetch(getHTTPAddress(), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  })
    .then((response) => response.json())
    .then((data) => {
      if (data['Error'] !== undefined) {
        cmd_reject(data['Error'])
      }
      cmd_resolve()
    })
    .catch((error) => {
      cmd_reject('HTTP Error: ' + error)
    })

  return new Promise((resolve, reject) => {
    cmd_resolve = resolve
    cmd_reject = reject
  })
}

/*
This is here to calculate the address. The dev environment is always on a different port to the daemon, so
we need to bounce requests across to the default port of the daemon. If we're running in production, we need
to convert the HTTP request to a websocket request on the same port (this can be changed), so work it out here.
 */
function getWebsocketAddress() {
  if (process.env.NODE_ENV === 'development') {
    return 'ws://localhost:14565/api/websocket'
  }
  return 'ws://' + window.location.host + '/api/websocket'
}

// Same as above, except for HTTP request...
function getHTTPAddress() {
  return getBaseHTTPAddress() + 'api/command'
}

export function getBaseHTTPAddress() {
  if (process.env.NODE_ENV === 'development') {
    return 'http://localhost:14565/'
  }
  return '/'
}
