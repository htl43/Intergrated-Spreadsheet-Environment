const Application = require('spectron').Application
const assert = require('assert')
const electronPath = require('electron') // Require Electron from the binaries included in node_modules.
const path = require('path')
fs = require('fs');

console.log(electronPath);
console.log([path.join(__dirname, '..', 'dist/main.js')]);
describe('Application launch', function () {
  this.timeout(20000)

  before(function () {
    this.app = new Application({
      // Your electron path can be any binary
      // i.e for OSX an example path could be '/Applications/MyApp.app/Contents/MacOS/MyApp'
      // But for the sake of the example we fetch it from our node_modules.
      path: electronPath,

      // Assuming you have the following directory structure

      //  |__ my project
      //     |__ ...
      //     |__ main.js
      //     |__ package.json
      //     |__ index.html
      //     |__ ...
      //     |__ test
      //        |__ spec.js  <- You are here! ~ Well you should be.

      // The following line tells spectron to look and use the main.js file
      // and the package.json located 1 level above.
      args: [path.join(__dirname, '..', 'dist/main.js')]
    })

    //many webdriverIO (app.client) methods not available on returned value
    return this.app.start().then(() => {
      this.app.client.waitUntilTextExists('#model').then(() => {
        this.model = JSON.stringify(this.app.getText("#model"));
        console.log(this.model);
      })
    })
  })
  // beforeEach(function () {
  //   this.app.client.click 
  // })
  console.log(this.model);
  after(function () {
    this.timeout(20000);
    console.log("model");
    console.log(this.model);
    console.log(this.app.client.getText("#model"));
    if (this.app && this.app.isRunning()) {
      return this.app.stop()
    }
  })

  it('shows an initial window', function () {
    return this.app.client.getWindowCount().then(function (count) {
      assert.equal(count, 2)
      // Please note that getWindowCount() will return 2 if `dev tools` are opened.
      // assert.equal(count, 2)
    })

  })

  it('Reset', function () {
    var temp = this.model;
    this.app.client.click('.Reset');
    this.app.client.waitUntilTextExists('#model').then(() => {
        this.model = JSON.stringify(this.app.getText("#model"));
        return assert.equal(temp, this.model)
      })

  })

  // it('shows Buttons', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('Toolbar Functions', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('Add Column', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('Add Row', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('Delete Column', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('Delete Row', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('ADD -> Delete Row / Column', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('ADD -> Delete Row & Column', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('Zoom In & Out', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('Zoom Reset', function () {
  //   return this.app.client.getWindowCount().then(function (count) {
  //     assert.equal(count, 2)
  //     // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //     // assert.equal(count, 2)
  //   })

  // })

  // it('Test Values', function () {
  //   return this.app.client.once('ready-to-show', function () {
  //     this.app.getText("model").then(function (text) {
  //       console.log(text);
  //       assert.equal(2, 2)
  //       // Please note that getWindowCount() will return 2 if `dev tools` are opened.
  //       // assert.equal(count, 2)
  //     })
  //   })
  // })

})