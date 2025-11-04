import {reactive} from 'vue'
import {applyOperation} from 'fast-json-patch'

export const store = reactive({
  is_connected: false,
  active: true,

  pausedPaths: [],

  // Set a 'base' status struct
  status: {
    config: {},
    audio: {
      profile: {},
    },
    devices: {
      Source: [],
      Target: [],
      applications: {},
    }
  },
  a11y: {
    notifications: {
      enabled: true,
      assertive: '',
      polite: ''
    }
  },

  socketDisconnected() {
    this.status = {
      config: {},
      audio: {
        profile: {},
      },
    }

    this.is_connected = false
  },

  socketConnected(status) {
    this.replaceData(status)
    this.is_connected = true
  },

  daemonVersion() {
    if (this.status !== undefined) {
      if (this.status.config !== undefined) {
        return this.status.config.daemon_version
      }
      return undefined
    } else {
      return undefined
    }
  },

  isConnected() {
    return this.is_connected
  },

  getConfig() {
    return this.status.config
  },

  getAudio() {
    return this.status.audio
  },

  getProfile() {
    return this.status.audio.profile
  },

  getDevices() {
    return this.status.devices;
  },

  getApplications() {
    return this.status.audio.applications;
  },

  getStatus() {
    return this.status;
  },

  replaceData(json) {
    if (this.active) {
      Object.assign(this.status, json.Status)
    }
  },

  pausePatchPath(path) {
    if (path === undefined) {
      console.error('Attempted to Stop Patches for Undefined!')
      return
    }
    let paths = path.split(';')
    for (path of paths) {
      console.log('Pausing Path: ' + path)
      this.pausedPaths.push(path)
    }
  },

  resumePatchPath(path) {
    let paths = path.split(';')
    for (path of paths) {
      let index = this.pausedPaths.indexOf(path)
      if (index !== -1) {
        // We don't care about key organisation, just that the entry is gone!
        delete this.pausedPaths[index]
      }
    }
  },

  // eslint-disable-next-line no-unused-vars
  patchData(json) {
    for (let patch of json.Patch) {
      if (this.pausedPaths.includes(patch.path)) {
        continue
      }

      applyOperation(this.status, patch, true, true, false)
    }

    // Trigger a resize event on our next frame, this ensures that things like the routing table
    // size changes don't mess with the rendering of the sliders.
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('resize'));
    });
  },

  pause() {
    this.active = false
  },

  resume() {
    this.active = true
  },

  isPaused() {
    return !this.active
  },

  getAccessibilityNotification(type) {
    if (this.a11y.notifications.enabled) {
      return this.a11y.notifications[type]
    }
    return ''
  },
  setAccessibilityNotification(type, message) {
    this.a11y.notifications[type] = message
  }
})
