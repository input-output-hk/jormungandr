import React from 'react';
import ListItem from '@material-ui/core/ListItem';
import ListItemIcon from '@material-ui/core/ListItemIcon';
import ListItemText from '@material-ui/core/ListItemText';
import ListSubheader from '@material-ui/core/ListSubheader';
import DashboardIcon from '@material-ui/icons/Dashboard';
import BarChartIcon from '@material-ui/icons/BarChart';
import SettingsApplicationsIcon from '@material-ui/icons/Layers';
import AssignmentIcon from '@material-ui/icons/Assignment';
import _ from 'lodash';

export function mainListItems(props) {
  const { history } = props;
  return (
    <div>
      <ListItem button onClick={() =>
        history.push({
          pathname: '/Dashboard/',
          state: history.location.state
        })
      }>
        <ListItemIcon>
          <DashboardIcon />
        </ListItemIcon>
        <ListItemText primary="Dashboard" />
      </ListItem>
    </div>);
}

export function secondaryListItems(secondaryMenu, history) {
  if (!secondaryMenu)
    return (<div />);

  var { menus, header } = secondaryMenu;
  let menuContent = menus.map(menuItem => renderSingleSecondaryMenuItem(menuItem, history));

  return (<div>
    <ListSubheader inset>{header}</ListSubheader>
    {menuContent}
  </div>)
}

function renderSingleSecondaryMenuItem(menuItem, history) {
  let { icon, text, path } = menuItem
  return (
    <ListItem button onClick={() =>
      history.push({
        pathname: path,
        state: history.location.state
      })
    }>
      <ListItemIcon >
        {icon}
      </ListItemIcon>
      <ListItemText primary={text} />
    </ListItem>)
}