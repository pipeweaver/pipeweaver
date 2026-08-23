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

function coerceValue(param, value) {
  if (typeof value === 'boolean' || value === 'true' || value === 'false') {
    return {"Bool": value === true || value === 'true'};
  } else if ('Int32' in param.value) {
    return {"Int32": parseInt(value)};
  } else if ('Float32' in param.value) {
    return {"Float32": parseFloat(value)};
  } else {
    return {"Float32": parseFloat(value)};
  }
}

export function setFilterValue(filterId, paramName, value) {
  const param = getFilterConfig(filterId).parameters.find(p => p.symbol === paramName);
  const id = parseInt(param.id);
  const send_value = coerceValue(param, value);

  return websocket.send_command({"SetFilterValue": [filterId, id, send_value]});
}

export function setFilterValues(filterId, values) {
  const config = getFilterConfig(filterId);

  const entries = values
    .map(({symbol, value}) => {
      const param = config.parameters.find(p => p.symbol === symbol);
      if (!param) return null;
      return {"id": parseInt(param.id), "value": coerceValue(param, value)};
    })
    .filter(entry => entry !== null);

  if (entries.length === 0) {
    return Promise.resolve();
  }

  return websocket.send_command({"SetFilterValues": [filterId, entries]});
}

export function getFilterConfig(filterId) {
  return store.getAudio().filter_config[filterId];
}
