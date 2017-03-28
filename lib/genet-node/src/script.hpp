#ifndef GENET_NODE_SCRIPT_H
#define GENET_NODE_SCRIPT_H

#include <memory>
#include <nan.h>

namespace genet_node {

class ArrayBufferAllocator;

class Script {
public:
  Script();
  virtual ~Script();

protected:
  v8::Isolate *isolate;
};

} // namespace genet_node

#endif
