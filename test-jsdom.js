import { JSDOM } from 'jsdom'

const dom = new JSDOM('<!DOCTYPE html><html><body></body></html>')
global.document = dom.window.document
global.window = dom.window

console.log('document type:', typeof document)
console.log('window type:', typeof window)
console.log('Testing jsdom works!')
