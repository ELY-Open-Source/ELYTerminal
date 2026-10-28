import './style.css';
import './orbs.css';
import { createDownloads } from './downloads.js';
import { createFrameScheduler } from './motion.js';
import { createPreferences } from './preferences.js';
import { createPanelTabs } from './scenes.js';
import { createType } from './type.js';

createPreferences();
createPanelTabs();
createDownloads();
createType();

const root = document.documentElement;
import('./stage.js')
  .then(({ createStage }) => createStage(document.getElementById('stage'), document.getElementById('ely'), document.getElementById('product-name')))
  .then((stage) => createFrameScheduler().add(stage.update))
  .catch((error) => {
    console.error('ELYTerminal: the swarm could not start', error);
    root.dataset.webgl = 'unavailable';
    root.dataset.swarm = 'ready';
  });
