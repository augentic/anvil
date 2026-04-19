package __ANDROID_PACKAGE__

import android.app.Application

class __APP_NAME__Application : Application() {
    override fun onCreate() {
        super.onCreate()
        System.setProperty("uniffi.component.shared.libraryOverride", "shared")
    }
}
