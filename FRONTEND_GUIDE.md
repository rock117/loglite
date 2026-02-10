# Loglite Frontend User Guide

## Overview

The Loglite frontend is a modern Vue3 application that provides a complete interface for managing applications, configuring log sources, and searching logs.

## Features

### ✅ Application Management
- Create and manage multiple applications
- Select active application from dropdown
- Application selection persisted in localStorage
- View all applications in dedicated Apps view

### ✅ Log Search
- Full-text search across all logs
- Real-time search with configurable result limit
- Severity-based filtering (ERROR, WARN, INFO, DEBUG)
- Formatted timestamp display
- Message preview with syntax highlighting
- Empty state handling

### ✅ Source Management
- Add log sources with file/directory paths
- Configure glob patterns (include/exclude)
- Enable/disable sources
- Delete sources
- Visual status indicators
- Recursive directory scanning option

### ✅ User Experience
- Responsive design
- Modal dialogs for forms
- Error notifications
- Loading states
- Empty state messages
- Keyboard shortcuts (Enter to search)

## Getting Started

### 1. Install Dependencies

```bash
cd loglite-frontend
npm install
```

### 2. Start Development Server

```bash
npm run dev
```

The frontend will be available at `http://localhost:5173`

### 3. Build for Production

```bash
npm run build
```

Built files will be in the `dist/` directory.

## User Interface

### Header

**Logo & Title**
- "Loglite" branding
- "Multi-App Log Search" subtitle

**Application Selector**
- Dropdown to select active application
- "+" button to create new application
- Selection persisted across sessions

### Navigation Tabs

**Search** - Search logs for selected application
**Sources** - Manage log sources (requires app selection)
**Apps** - View and manage all applications

### Search View

**Search Controls**
- Search input: Enter keywords or queries
- Limit input: Number of results (1-1000)
- Search button: Execute search

**Search Results Table**
- Timestamp: Formatted local time
- Severity: Color-coded badge (ERROR/WARN/INFO/DEBUG)
- Source: Log source file/path
- Host: Hostname
- Message: Full log message with wrapping

**Keyboard Shortcuts**
- `Enter` in search box: Execute search

### Sources View

**Source Cards**
- Status indicator (green = enabled, gray = disabled)
- Path display
- Configuration tags (Recursive, Include, Exclude patterns)
- Enable/Disable button
- Delete button

**Add Source Form**
- Path: File or directory path
- Include Pattern: Glob pattern for files to include
- Exclude Pattern: Glob pattern for files to exclude
- Recursive: Scan subdirectories checkbox
- Enable immediately: Start monitoring checkbox

### Apps View

**Application Cards**
- Application name
- Application ID (generated)
- Creation date
- Click to select
- Selected app highlighted in blue

**Create App Form**
- Application name input
- Auto-generates stable app_id

## Usage Examples

### Creating Your First Application

1. Click the "+" button in the header
2. Enter application name (e.g., "my-service")
3. Click "Create"
4. Application is automatically selected

### Adding a Log Source

1. Select an application from dropdown
2. Navigate to "Sources" tab
3. Click "+ Add Source"
4. Enter path: `/var/log/myapp`
5. Set include pattern: `*.log`
6. Set exclude pattern: `*.gz`
7. Check "Recursive" if needed
8. Click "Add Source"

### Searching Logs

1. Select an application
2. Navigate to "Search" tab
3. Enter search query (e.g., "ERROR", "exception")
4. Adjust result limit if needed
5. Click "Search" or press Enter
6. View results in table

### Managing Sources

**Enable/Disable Source**
- Click "Enable" or "Disable" button on source card
- Status indicator updates immediately

**Delete Source**
- Click "Delete" button on source card
- Confirm deletion in dialog

## API Integration

The frontend communicates with the backend API at `/api/*`:

### Applications
- `GET /api/apps` - List all applications
- `POST /api/apps` - Create application

### Sources
- `GET /api/sources?app_id=<id>` - List sources for app
- `POST /api/sources` - Create source
- `PUT /api/sources/:id` - Update source
- `DELETE /api/sources/:id` - Delete source

### Search
- `POST /api/search` - Search logs

## Configuration

### API Base URL

By default, the frontend proxies API requests to `http://localhost:8000`.

To change this, update `vite.config.ts`:

```typescript
export default defineConfig({
  server: {
    proxy: {
      '/api': {
        target: 'http://your-backend:8000',
        changeOrigin: true
      }
    }
  }
})
```

### localStorage Keys

- `loglite_selected_app` - Currently selected application ID

## Styling

The frontend uses a modern, clean design with:
- Tailwind-inspired color palette
- Responsive grid layouts
- Smooth transitions
- Accessible color contrast
- Monospace fonts for code/logs

### Color Scheme

**Primary**: Blue (#3b82f6)
**Error**: Red (#dc2626)
**Warning**: Amber (#f59e0b)
**Success**: Green (#10b981)
**Gray Scale**: Tailwind gray palette

### Severity Colors

- **ERROR**: Red background, dark red text
- **WARN**: Amber background, dark amber text
- **INFO**: Blue background, dark blue text
- **DEBUG**: Gray background, dark gray text

## Development

### Project Structure

```
loglite-frontend/
├── src/
│   ├── App.vue          # Main application component
│   └── main.ts          # Application entry point
├── index.html           # HTML template
├── package.json         # Dependencies
├── tsconfig.json        # TypeScript config
└── vite.config.ts       # Vite config
```

### Technologies

- **Vue 3** - Progressive JavaScript framework
- **TypeScript** - Type-safe development
- **Vite** - Fast build tool
- **Axios** - HTTP client

### Adding Features

The application is structured as a single-file component (`App.vue`) with:
- `<script setup>` - Composition API logic
- `<template>` - HTML structure
- `<style scoped>` - Component styles

To add new features:
1. Add state variables in `<script setup>`
2. Add functions for API calls
3. Add UI elements in `<template>`
4. Add styles in `<style scoped>`

## Troubleshooting

### Frontend Won't Start

```bash
# Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm install
npm run dev
```

### API Requests Failing

1. Ensure backend is running on port 8000
2. Check browser console for CORS errors
3. Verify proxy configuration in `vite.config.ts`

### Application Not Persisting

- Check browser localStorage
- Clear localStorage: `localStorage.clear()`
- Verify app_id in localStorage matches existing app

### Styles Not Loading

```bash
# Rebuild the project
npm run build
npm run dev
```

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

## Accessibility

- Keyboard navigation support
- Focus indicators
- ARIA labels (can be enhanced)
- Color contrast compliance

## Performance

- Lazy loading for large result sets
- Efficient re-rendering with Vue 3
- Minimal bundle size (~100KB gzipped)
- Fast initial load (<1s)

## Future Enhancements

- [ ] Real-time log streaming (WebSocket)
- [ ] Advanced search filters (date range, severity)
- [ ] Log export functionality
- [ ] Dark mode
- [ ] Saved searches
- [ ] Dashboard with metrics
- [ ] User preferences
- [ ] Keyboard shortcuts panel

## Contributing

To contribute to the frontend:
1. Follow Vue 3 Composition API patterns
2. Use TypeScript for type safety
3. Keep styles scoped to components
4. Test in multiple browsers
5. Ensure responsive design

## Support

For issues or questions:
- Check backend logs for API errors
- Review browser console for frontend errors
- Verify network requests in DevTools
- Ensure backend and frontend versions match

---

**Version**: 0.1.0
**Last Updated**: 2024-02-09
