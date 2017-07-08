#include "extended_slot.hpp"
#include "plugkit_module.hpp"
#include "private/variant.hpp"
#include "session.hpp"
#include <nan.h>

#if defined(PLUGKIT_OS_LINUX)
#include <dlfcn.h>
#endif

using namespace plugkit;

NAN_GC_CALLBACK(gcPrologueCallback) {
  Variant::Private::cleanupSharedBuffers();
}

namespace {
void Init(v8::Local<v8::Object> exports) {
#if defined(PLUGKIT_OS_LINUX)
  Dl_info info;
  if (dladdr(reinterpret_cast<const void *>(&Init), &info) != 0) {
    dlopen(info.dli_fname, RTLD_LAZY | RTLD_NOLOAD | RTLD_GLOBAL);
  }
#endif

  Nan::AddGCPrologueCallback(gcPrologueCallback);

  v8::Isolate *isolate = v8::Isolate::GetCurrent();
  ExtendedSlot::init(isolate);
  new PlugkitModule(isolate, exports, true);
}
}

NODE_MODULE(plugkit, Init)
