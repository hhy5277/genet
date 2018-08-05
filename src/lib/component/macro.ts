import BaseComponent from './base'
import { Disposable } from '../disposable'
import Script from '../script'
import genet from '@genet/api'
import objpath from 'object-path'
import path from 'path'

export default class MacroComponent implements BaseComponent {
  private name: string
  private description: string
  private mainFile: string
  private disposable: Disposable

  constructor(comp: any, dir: string) {
    const file = objpath.get(comp, 'main', '')
    if (!file) {
      throw new Error('main field required')
    }
    this.mainFile = path.resolve(dir, file)
    this.name = objpath.get(comp, 'name', '')
    this.description = objpath.get(comp, 'description', '')
  }
  async load() {
    const module = await Script.execute(this.mainFile)
    this.disposable = genet.session.registerFilterMacro({
      name: this.name,
      description: this.description,
      func: module,
    })
    return true
  }
  async unload() {
    if (this.disposable) {
      this.disposable.dispose()
    }
    return true
  }
}