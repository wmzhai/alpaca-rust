import test from 'node:test';
import assert from 'node:assert/strict';

import { analysis, pricing } from '@alpaca/option';
import * as mathAmerican from '@alpaca/option/math/american';
import * as mathBachelier from '@alpaca/option/math/bachelier';
import * as mathBarrier from '@alpaca/option/math/barrier';
import * as mathBlack76 from '@alpaca/option/math/black76';
import * as mathGeometricAsian from '@alpaca/option/math/geometric-asian';

test('package exports expose public core and approved math subpaths', () => {
  assert.equal(typeof analysis.otmPercent, 'function');
  assert.equal(typeof pricing.priceBlackScholes, 'function');
  assert.equal(typeof mathAmerican.treePrice, 'function');
  assert.equal(typeof mathBachelier.price, 'function');
  assert.equal(typeof mathBarrier.price, 'function');
  assert.equal(typeof mathBlack76.price, 'function');
  assert.equal(typeof mathGeometricAsian.price, 'function');
});
