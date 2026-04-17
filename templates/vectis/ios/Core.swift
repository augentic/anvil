import Foundation
import Shared
import SharedTypes

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    private let core: CoreFfi

    init() {
        self.core = CoreFfi()
        do {
            self.view = Self.deserializeView(try core.view())
        } catch {
            assertionFailure("Failed to get initial view from core: \(error)")
            self.view = .loading
        }
    }

    func update(_ event: Event) {
        guard let data = try? event.bincodeSerialize() else {
            assertionFailure("Failed to serialize event: \(event)")
            return
        }
        do {
            let effects = try core.update(data: Data(data))
            processEffects([UInt8](effects))
        } catch {
            assertionFailure("Failed to update core: \(error)")
        }
    }

    private func processEffects(_ data: [UInt8]) {
        guard let requests = try? [Request].bincodeDeserialize(input: data) else {
            assertionFailure("Failed to deserialize requests")
            return
        }
        for request in requests {
            processEffect(request)
        }
    }

    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            do {
                let data = try core.view()
                guard let vm = try? ViewModel.bincodeDeserialize(input: [UInt8](data)) else {
                    assertionFailure("Failed to deserialize ViewModel from bincode")
                    break
                }
                self.view = vm
            } catch {
                assertionFailure("Failed to get view from core: \(error)")
            }
<<<CAP:http
        case .http(let httpRequest):
            Task { @MainActor in
                let response = await Self.performHttpRequest(httpRequest)
                guard let data = try? HttpResult.ok(response).bincodeSerialize() else {
                    assertionFailure("Failed to serialize HttpResult")
                    return
                }
                do {
                    let effects = try core.resolve(id: request.id, data: Data(data))
                    processEffects([UInt8](effects))
                } catch {
                    assertionFailure("Failed to resolve effect: \(error)")
                }
            }
CAP:http>>>
<<<CAP:kv
        case .keyValue(let kvOp):
            // TODO: implement key-value persistence (writer skill, Update Mode)
            _ = kvOp
CAP:kv>>>
<<<CAP:time
        case .time(let timeRequest):
            // TODO: implement time handling (writer skill, Update Mode)
            _ = timeRequest
CAP:time>>>
<<<CAP:platform
        case .platform(let platformRequest):
            // TODO: implement platform info (writer skill, Update Mode)
            _ = platformRequest
CAP:platform>>>
        }
    }

    /// Only used during `init()` where `.loading` is the correct fallback.
    /// The `.render` handler preserves the existing view on failure.
    private static func deserializeView(_ data: Data) -> ViewModel {
        guard let vm = try? ViewModel.bincodeDeserialize(input: [UInt8](data)) else {
            assertionFailure("Failed to deserialize ViewModel from bincode")
            return .loading
        }
        return vm
    }
<<<CAP:http

    private static func performHttpRequest(_ request: HttpRequest) async -> HttpResponse {
        guard let url = URL(string: request.url) else {
            return HttpResponse(status: 0, headers: [], body: [])
        }
        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = request.method

        for header in request.headers {
            urlRequest.setValue(header.value, forHTTPHeaderField: header.name)
        }

        if !request.body.isEmpty {
            urlRequest.httpBody = Data(request.body)
        }

        do {
            let (data, response) = try await URLSession.shared.data(for: urlRequest)
            guard let httpResponse = response as? HTTPURLResponse else {
                return HttpResponse(status: 0, headers: [], body: [])
            }
            return HttpResponse(
                status: UInt16(httpResponse.statusCode),
                headers: httpResponse.allHeaderFields.map { key, value in
                    HttpHeader(
                        name: String(describing: key),
                        value: String(describing: value)
                    )
                },
                body: [UInt8](data)
            )
        } catch {
            return HttpResponse(status: 0, headers: [], body: [])
        }
    }
CAP:http>>>
}
