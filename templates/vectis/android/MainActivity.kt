package __ANDROID_PACKAGE__

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import __ANDROID_PACKAGE__.core.Core
import __ANDROID_PACKAGE__.ui.screens.HomeScreen
import __ANDROID_PACKAGE__.ui.screens.LoadingScreen
import __ANDROID_PACKAGE__.ui.theme.AppTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AppTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    AppView()
                }
            }
        }
    }
}

@Composable
fun AppView(core: Core = viewModel()) {
    when (val state = core.view) {
        is ViewModel.Loading -> LoadingScreen()
        is ViewModel.Home -> HomeScreen(
            viewModel = state.value,
            onEvent = { core.update(it) }
        )
    }
}
