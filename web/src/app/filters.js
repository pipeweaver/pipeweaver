import {websocket} from "@/app/sockets.js";
import {store} from "@/app/store.js";

const MINIMUM_DB = -80.0;
const MAXIMUM_DB = 20.0;

export function dbToLinear(db) {
  return Math.exp((db / 20.0) * Math.LN10);
}

export function linearToDb(amp) {
  if (amp <= 0) return MINIMUM_DB;
  return Math.min(MAXIMUM_DB, Math.max(MINIMUM_DB, 20.0 * Math.log10(amp)));
}

export function setParameterValue(filterId, paramName, value) {
  const param = getFilterConfig(filterId).parameters.find(p => p.symbol === paramName);
  const id = parseInt(param.id);

  let send_value;
  if (typeof value === 'boolean' || value === 'true' || value === 'false') {
    send_value = {"Bool": value === true || value === 'true'};
  } else if ('Int32' in param.value) {
    send_value = {"Int32": parseInt(value)};
  } else if ('Float32' in param.value) {
    send_value = {"Float32": parseFloat(value)};
  } else {
    send_value = {"Float32": parseFloat(value)};
  }

  websocket.send_command({"SetFilterValue": [filterId, id, send_value]});
}

export function getFilterConfig(filterId) {
  return store.getAudio().filter_config[filterId];
}
