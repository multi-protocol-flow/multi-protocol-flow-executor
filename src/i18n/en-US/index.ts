import type { Translation } from '../i18n-types';
import common from './common';
import nodes from './nodes';
import panels from './panels';
import execution from './execution';
import settings from './settings';
import flow from './flow';
import validation from './validation';
import welcome from './welcome';
import shortcuts from './shortcuts';

const en_US: Translation = {
  common: common as Translation['common'],
  nodes: nodes as Translation['nodes'],
  panels: panels as Translation['panels'],
  execution: execution as Translation['execution'],
  settings: settings as Translation['settings'],
  flow: flow as Translation['flow'],
  validation: validation as Translation['validation'],
  welcome: welcome as Translation['welcome'],
  shortcuts: shortcuts as Translation['shortcuts'],
};

export default en_US;

