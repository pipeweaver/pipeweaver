import './assets/main.css'
import {createApp} from 'vue'
import App from './App.vue'

/* import the fontawesome core */
/* import font awesome icon component */
import {library} from "@fortawesome/fontawesome-svg-core";
import {FontAwesomeIcon} from '@fortawesome/vue-fontawesome'

import {
  faAngleDown,
  faCircleCheck,
  faVolumeHigh,
  faVolumeXmark,
  faXmark,
} from "@fortawesome/free-solid-svg-icons";

library.add(faVolumeHigh, faVolumeXmark, faAngleDown, faCircleCheck, faXmark);

const app = createApp(App);
app.component('font-awesome-icon', FontAwesomeIcon);

app.mount('#app');
