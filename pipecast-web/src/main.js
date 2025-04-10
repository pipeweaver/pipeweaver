import './assets/main.css'
import {createApp} from 'vue'
import App from './App.vue'
import vClickOutside from "click-outside-vue3";

/* import the fontawesome core */
/* import font awesome icon component */
import {library} from "@fortawesome/fontawesome-svg-core";
import {FontAwesomeIcon} from '@fortawesome/vue-fontawesome'

import {
  faAngleDown,
  faCheck,
  faCircleCheck,
  faVolumeHigh,
  faVolumeXmark,
  faXmark,
} from "@fortawesome/free-solid-svg-icons";

library.add(faVolumeHigh, faVolumeXmark, faAngleDown, faCircleCheck, faXmark, faCheck);

const app = createApp(App);
app.component('font-awesome-icon', FontAwesomeIcon);
app.use(vClickOutside)

app.mount('#app');
