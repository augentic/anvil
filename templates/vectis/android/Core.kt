package __ANDROID_PACKAGE__.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
<<<CAP:http
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch
CAP:http>>>
import __ANDROID_PACKAGE__.Effect
import __ANDROID_PACKAGE__.Event
import __ANDROID_PACKAGE__.Request
import __ANDROID_PACKAGE__.Requests
import __ANDROID_PACKAGE__.ViewModel
<<<CAP:http
import __ANDROID_PACKAGE__.HttpError
import __ANDROID_PACKAGE__.HttpRequest
import __ANDROID_PACKAGE__.HttpResponse
import __ANDROID_PACKAGE__.HttpResult
CAP:http>>>
import uniffi.shared.CoreFfi

open class Core : androidx.lifecycle.ViewModel() {
    private val coreFfi: CoreFfi = CoreFfi()

    var view: ViewModel by mutableStateOf(
        ViewModel.bincodeDeserialize(coreFfi.view())
    )
        private set

    fun update(event: Event) {
        val effects = coreFfi.update(event.bincodeSerialize())
        handleEffects(effects)
    }

    private fun handleEffects(effects: ByteArray) {
        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processRequest(request)
        }
    }

    private fun processRequest(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(coreFfi.view())
            }
<<<CAP:http
            is Effect.Http -> {
                viewModelScope.launch {
                    val result = performHttpRequest(effect.value)
                    resolveAndHandleEffects(request.id, result.bincodeSerialize())
                }
            }
CAP:http>>>
<<<CAP:kv
            is Effect.KeyValue -> {
                @Suppress("UNUSED_VARIABLE")
                val keyValueOp = effect.value
            }
CAP:kv>>>
<<<CAP:time
            is Effect.Time -> {
                @Suppress("UNUSED_VARIABLE")
                val timeRequest = effect.value
            }
CAP:time>>>
<<<CAP:platform
            is Effect.Platform -> {
                @Suppress("UNUSED_VARIABLE")
                val platformRequest = effect.value
            }
CAP:platform>>>
        }
    }

<<<CAP:http
    private fun resolveAndHandleEffects(requestId: UInt, data: ByteArray) {
        val effects = coreFfi.resolve(requestId, data)
        handleEffects(effects)
    }

    private suspend fun performHttpRequest(request: HttpRequest): HttpResult {
        @Suppress("UNUSED_VARIABLE")
        val req = request
        return HttpResult.Err(
            HttpError.Io("HTTP not implemented in deterministic baseline; writer skill enables in Update Mode")
        )
    }
CAP:http>>>
}
