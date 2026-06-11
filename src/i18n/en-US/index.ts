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
import viewer from './viewer';

const en_US: Translation = {
  common,
  nodes,
  panels,
  execution,
  settings,
  flow,
  validation,
  welcome,
  shortcuts,
  viewer,
} as Translation;

export default en_US;
