# Jetpack Compose View Patterns for Crux Apps

Patterns for building Jetpack Compose UI that consumes Crux `ViewModel` data
and dispatches `Event` values back to the core.

## Root Composable: ViewModel Switch

The root composable switches on the `ViewModel` sealed class to display the
appropriate screen. This is the main dispatch point.

### Pattern 1: ViewModel-based Core (simple apps)

```kotlin
@Composable
fun AppView(core: Core = viewModel()) {
    when (val state = core.view) {
        is ViewModel.Loading -> LoadingScreen()
        is ViewModel.Main -> MainScreen(
            viewModel = state.value,
            onEvent = { core.update(it) }
        )
        is ViewModel.Error -> ErrorScreen(
            viewModel = state.value,
            onEvent = { core.update(it) }
        )
    }
}
```

### Pattern 2: StateFlow-based Core (complex apps with DI)

```kotlin
@Composable
fun AppView(
    state: ViewModel,
    onEvent: (Event) -> Unit
) {
    when (state) {
        is ViewModel.Loading -> LoadingScreen()
        is ViewModel.Main -> MainScreen(
            viewModel = state.value,
            onEvent = onEvent
        )
        is ViewModel.Error -> ErrorScreen(
            viewModel = state.value,
            onEvent = onEvent
        )
    }
}

// In MainActivity:
setContent {
    AppTheme {
        val state by core.viewModel.collectAsState()
        Surface(
            modifier = Modifier.fillMaxSize(),
            color = MaterialTheme.colorScheme.background
        ) {
            AppView(state = state, onEvent = { core.update(it) })
        }
    }
}
```

### Rules

- The `when` must be exhaustive -- one branch per `ViewModel` variant.
- Pass view model data down as a value, not the `Core` reference.
- Pass an event callback (`(Event) -> Unit`) for user interactions.
- Composables are pure functions of their input -- no direct core access.

## Screen Pattern

Each screen is a standalone composable that receives its data and an event
callback. Screens correspond 1:1 to `ViewModel` variants.

```kotlin
@Composable
fun MainScreen(
    viewModel: MainView,
    onEvent: (Event) -> Unit
) {
    Scaffold(
        topBar = {
            TopAppBar(title = { Text("Items") })
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { onEvent(Event.AddItem("New Item")) }
            ) {
                Icon(
                    Icons.Default.Add,
                    contentDescription = "Add item"
                )
            }
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            items(viewModel.items, key = { it.id }) { item ->
                ItemRow(
                    item = item,
                    onToggle = { onEvent(Event.ToggleItem(item.id)) }
                )
            }
        }
    }
}
```

## Loading Screen

A simple centered indicator. No data needed from the core.

```kotlin
@Composable
fun LoadingScreen() {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            CircularProgressIndicator()
            Text(
                text = "Loading...",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}
```

## Error Screen

Displays an error message with an optional retry button.

```kotlin
@Composable
fun ErrorScreen(
    viewModel: ErrorView,
    onEvent: (Event) -> Unit
) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(24.dp),
            modifier = Modifier.padding(horizontal = 32.dp)
        ) {
            Icon(
                imageVector = Icons.Default.Warning,
                contentDescription = null,
                modifier = Modifier.size(56.dp),
                tint = MaterialTheme.colorScheme.error
            )
            Text(
                text = viewModel.message,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurface,
                textAlign = TextAlign.Center
            )
            if (viewModel.canRetry) {
                Button(
                    onClick = { onEvent(Event.Navigate(Route.MAIN)) },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.primary
                    )
                ) {
                    Text("Try Again")
                }
            }
        }
    }
}
```

## List Rendering

Use `LazyColumn` with `items` for lists from the view model.

```kotlin
LazyColumn(
    modifier = Modifier.fillMaxSize()
) {
    items(viewModel.items, key = { it.id }) { item ->
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clickable { onEvent(Event.ToggleItem(item.id)) }
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Icon(
                imageVector = if (item.completed)
                    Icons.Default.CheckCircle
                else
                    Icons.Outlined.Circle,
                contentDescription = if (item.completed)
                    "Completed" else "Not completed",
                tint = if (item.completed)
                    MaterialTheme.colorScheme.primary
                else
                    MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                text = item.title,
                style = MaterialTheme.typography.bodyLarge,
                textDecoration = if (item.completed)
                    TextDecoration.LineThrough else TextDecoration.None
            )
        }
    }
}
```

## Form Inputs

For text input, use `remember { mutableStateOf("") }` for the local editing
buffer and dispatch an event on submit.

```kotlin
@Composable
fun AddItemField(onEvent: (Event) -> Unit) {
    var text by remember { mutableStateOf("") }

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        OutlinedTextField(
            value = text,
            onValueChange = { text = it },
            modifier = Modifier.weight(1f),
            label = { Text("New item") },
            singleLine = true,
            keyboardActions = KeyboardActions(
                onDone = {
                    if (text.isNotBlank()) {
                        onEvent(Event.AddItem(text.trim()))
                        text = ""
                    }
                }
            )
        )
        Button(
            onClick = {
                if (text.isNotBlank()) {
                    onEvent(Event.AddItem(text.trim()))
                    text = ""
                }
            },
            enabled = text.isNotBlank()
        ) {
            Text("Add")
        }
    }
}
```

## Navigation with Route

When the Crux core defines a `Route` enum, navigation events are dispatched
as `Event.Navigate(route)`. Compose Navigation or simple state-based switching
can be used.

### Tab Navigation

```kotlin
@Composable
fun AppView(
    state: ViewModel,
    onEvent: (Event) -> Unit
) {
    var selectedTab by remember { mutableIntStateOf(0) }

    Scaffold(
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    selected = selectedTab == 0,
                    onClick = {
                        selectedTab = 0
                        onEvent(Event.Navigate(Route.MAIN))
                    },
                    icon = { Icon(Icons.Default.Home, contentDescription = "Home") },
                    label = { Text("Home") }
                )
                NavigationBarItem(
                    selected = selectedTab == 1,
                    onClick = {
                        selectedTab = 1
                        onEvent(Event.Navigate(Route.SETTINGS))
                    },
                    icon = { Icon(Icons.Default.Settings, contentDescription = "Settings") },
                    label = { Text("Settings") }
                )
            }
        }
    ) { padding ->
        Box(modifier = Modifier.padding(padding)) {
            // render current view based on state
        }
    }
}
```

## Swipe Actions

Use `SwipeToDismissBox` for swipe-to-delete:

```kotlin
val dismissState = rememberSwipeToDismissBoxState(
    confirmValueChange = {
        if (it == SwipeToDismissBoxValue.EndToStart) {
            onEvent(Event.DeleteItem(item.id))
            true
        } else false
    }
)

SwipeToDismissBox(
    state = dismissState,
    backgroundContent = {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.error)
                .padding(horizontal = 20.dp),
            contentAlignment = Alignment.CenterEnd
        ) {
            Icon(
                Icons.Default.Delete,
                contentDescription = "Delete",
                tint = MaterialTheme.colorScheme.onError
            )
        }
    }
) {
    // Item content
}
```

## Pull-to-Refresh

```kotlin
val pullRefreshState = rememberPullToRefreshState()

PullToRefreshBox(
    isRefreshing = false,
    onRefresh = { onEvent(Event.REFRESH) },
    state = pullRefreshState
) {
    LazyColumn { /* items */ }
}
```

## Status Indicators

For sync status or connectivity indicators within a screen:

```kotlin
if (viewModel.syncStatus.isNotEmpty()) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            Icons.Default.Sync,
            contentDescription = null,
            modifier = Modifier.size(16.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            text = viewModel.syncStatus,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}
```

## Accessibility

- Use `contentDescription` for icons and non-text interactive elements.
- Use `semantics { }` for custom accessibility info.
- Mark decorative icons with `contentDescription = null`.

```kotlin
Icon(
    imageVector = Icons.Default.CheckCircle,
    contentDescription = if (item.completed) "Completed" else "Not completed"
)
```

## Preview Provider

Every screen composable should have a preview with sample data:

```kotlin
@Preview(showBackground = true)
@Composable
fun MainScreenPreview() {
    AppTheme {
        MainScreen(
            viewModel = MainView(
                items = listOf(
                    ItemView(id = "1", title = "Sample Item", completed = false),
                    ItemView(id = "2", title = "Done Item", completed = true),
                ),
                itemCount = "2 items"
            ),
            onEvent = { }
        )
    }
}

@Preview(showBackground = true)
@Composable
fun LoadingScreenPreview() {
    AppTheme {
        LoadingScreen()
    }
}

@Preview(showBackground = true)
@Composable
fun ErrorScreenPreview() {
    AppTheme {
        ErrorScreen(
            viewModel = ErrorView(
                message = "Failed to connect to server.",
                canRetry = true
            ),
            onEvent = { }
        )
    }
}
```

## Material 3 Theme

### Color.kt

```kotlin
package com.vectis.myapp.ui.theme

import androidx.compose.ui.graphics.Color

val Purple80 = Color(0xFFD0BCFF)
val PurpleGrey80 = Color(0xFFCCC2DC)
val Pink80 = Color(0xFFEFB8C8)

val Purple40 = Color(0xFF6650a4)
val PurpleGrey40 = Color(0xFF625b71)
val Pink40 = Color(0xFF7D5260)
```

### Theme.kt

```kotlin
package com.vectis.myapp.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

private val DarkColorScheme = darkColorScheme(
    primary = Purple80,
    secondary = PurpleGrey80,
    tertiary = Pink80
)

private val LightColorScheme = lightColorScheme(
    primary = Purple40,
    secondary = PurpleGrey40,
    tertiary = Pink40
)

@Composable
fun AppTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context)
            else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
```

### Type.kt

```kotlin
package com.vectis.myapp.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

val Typography = Typography(
    bodyLarge = TextStyle(
        fontFamily = FontFamily.Default,
        fontWeight = FontWeight.Normal,
        fontSize = 16.sp,
        lineHeight = 24.sp,
        letterSpacing = 0.5.sp
    )
)
```
