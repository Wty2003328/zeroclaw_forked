import { Card, CardContent } from '../ui/card';
import { Button } from '../ui/button';

interface Props {
  onToast: (message: string, type: 'success' | 'error') => void;
}

export function GeneralPanel({ onToast }: Props) {
  const handleResetLayout = () => {
    localStorage.removeItem('dashboard-layout-v8');
    onToast('Dashboard layout reset. Reload the page to apply.', 'success');
  };

  return (
    <div>
      <h2 className="text-lg font-semibold mb-1">General</h2>
      <p className="text-sm text-muted-foreground mb-6">Dashboard appearance and behavior.</p>

      <div className="space-y-6">
        {/* Theme */}
        <Card>
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-medium">Theme</h3>
                <p className="text-xs text-muted-foreground mt-0.5">Dashboard color scheme</p>
              </div>
              <select
                defaultValue="dark"
                disabled
                className="h-8 rounded-md border border-input bg-transparent px-3 text-sm text-muted-foreground"
              >
                <option value="dark">Dark</option>
              </select>
            </div>
          </CardContent>
        </Card>

        {/* Reset Layout */}
        <Card>
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-medium">Dashboard Layout</h3>
                <p className="text-xs text-muted-foreground mt-0.5">Reset widget positions and sizes to default</p>
              </div>
              <Button variant="outline" size="sm" onClick={handleResetLayout}>Reset Layout</Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
