import { combineReducers } from 'redux';
import DashboardReducer from './user/dashboard/reducers';
import ChannelReducers from './user/channel/show/reducers';
import NewChannelReducer from './user/channel/new/reducers';
import ChannelSettingsReducer from './user/channel/settings/reducers';

export default combineReducers({
  dashboard: DashboardReducer,
  channel: ChannelReducers,
  newChannel: NewChannelReducer,
  settings: ChannelSettingsReducer
});