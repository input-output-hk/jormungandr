import React from 'react';
import PropTypes from 'prop-types';
import { withStyles } from '@material-ui/core/styles';
import Table from '@material-ui/core/Table';
import TableBody from '@material-ui/core/TableBody';
import TableCell from '@material-ui/core/TableCell';
import TableHead from '@material-ui/core/TableHead';
import TableRow from '@material-ui/core/TableRow';
import Paper from '@material-ui/core/Paper';
import MaterialTable from 'material-table';
import { styles } from '../styles';
import { makeStyles } from '@material-ui/core/styles';
import { forwardRef } from 'react';
import AddBox from '@material-ui/icons/AddBox';
import ArrowUpward from '@material-ui/icons/ArrowUpward';
import Check from '@material-ui/icons/Check';
import ChevronLeft from '@material-ui/icons/ChevronLeft';
import ChevronRight from '@material-ui/icons/ChevronRight';
import Clear from '@material-ui/icons/Clear';
import DeleteOutline from '@material-ui/icons/DeleteOutline';
import Edit from '@material-ui/icons/Edit';
import FilterList from '@material-ui/icons/FilterList';
import FirstPage from '@material-ui/icons/FirstPage';
import LastPage from '@material-ui/icons/LastPage';
import Remove from '@material-ui/icons/Remove';
import SaveAlt from '@material-ui/icons/SaveAlt';
import Search from '@material-ui/icons/Search';
import ViewColumn from '@material-ui/icons/ViewColumn';

const tableIcons = {
  Add: forwardRef((props, ref) => <AddBox {...props} ref={ref} />),
  Check: forwardRef((props, ref) => <Check {...props} ref={ref} />),
  Clear: forwardRef((props, ref) => <Clear {...props} ref={ref} />),
  Delete: forwardRef((props, ref) => <DeleteOutline {...props} ref={ref} />),
  DetailPanel: forwardRef((props, ref) => <ChevronRight {...props} ref={ref} />),
  Edit: forwardRef((props, ref) => <Edit {...props} ref={ref} />),
  Export: forwardRef((props, ref) => <SaveAlt {...props} ref={ref} />),
  Filter: forwardRef((props, ref) => <FilterList {...props} ref={ref} />),
  FirstPage: forwardRef((props, ref) => <FirstPage {...props} ref={ref} />),
  LastPage: forwardRef((props, ref) => <LastPage {...props} ref={ref} />),
  NextPage: forwardRef((props, ref) => <ChevronRight {...props} ref={ref} />),
  PreviousPage: forwardRef((props, ref) => <ChevronLeft {...props} ref={ref} />),
  ResetSearch: forwardRef((props, ref) => <Clear {...props} ref={ref} />),
  Search: forwardRef((props, ref) => <Search {...props} ref={ref} />),
  SortArrow: forwardRef((props, ref) => <ArrowUpward {...props} ref={ref} />),
  ThirdStateCheck: forwardRef((props, ref) => <Remove {...props} ref={ref} />),
  ViewColumn: forwardRef((props, ref) => <ViewColumn {...props} ref={ref} />)
};

function SimpleTable(props) {
  const [state, setState] = React.useState({
    columns: [
      { title: 'Created At time', field: 'created_at_time', type: 'date' },
      { title: 'Leader Id', field: 'enclave_leader_id', type: 'numeric' },
      { title: 'Finished at time', field: 'finished_at_time', type: 'date' },

      { title: 'Scheduled at date', field: 'scheduled_at_date', },
      { title: 'Scheduled at time', field: 'scheduled_at_time', type: 'date' },
      { title: 'Wake at time', field: 'wake_at_time', type: 'date' },
    ],
    data: props.chartData.map(
      row => ({
        created_at_time: convert_to_date(row.created_at_time),
        enclave_leader_id: row.enclave_leader_id,
        finished_at_time: convert_to_date(row.finished_at_time),
        scheduled_at_date: row.scheduled_at_date,
        scheduled_at_time: convert_to_date(row.scheduled_at_time),
        wake_at_time: convert_to_date(row.wake_at_time),
      })
    )
  });
  const classes = useStyles();
  return (
    <Paper className={classes.root}>
      <MaterialTable className={classes.table}
        icons={tableIcons}
        title="LeaderShip Log"
        columns={state.columns}
        data={state.data}
      />
    </Paper>
  );
}

const useStyles = makeStyles(theme => ({
  root: {
    width: '100%',
    marginTop: theme.spacing(3),
    overflowX: 'auto',
  },
  table: {
    minWidth: 650,
  },
}));

/*
function SimpleTable(props) {
const {classes, chartData } = props;
  return (
<Paper className={classes.root}>
    <Table className={classes.table}>
      <TableHead>
        <TableRow>
          <TableCell>Created At time</TableCell>
          <TableCell numeric>Leader Id</TableCell>
          <TableCell>Finished At time</TableCell>
          <TableCell>Scheduled at date </TableCell>
          <TableCell>Scheduled at time </TableCell>
          <TableCell>Wake at time </TableCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {chartData.map(n => {
          return (
            <TableRow key={n.id}>
              <TableCell component="th" scope="row">
                {convert_to_date(n.created_at_time)}
              </TableCell>
              <TableCell numeric>{n.enclave_leader_id}</TableCell>
              <TableCell>{convert_to_date(n.finished_at_time)}</TableCell>
              <TableCell>{n.scheduled_at_date} </TableCell>
              <TableCell>{convert_to_date(n.scheduled_at_time)}</TableCell>
              <TableCell>{convert_to_date(n.wake_at_time)}</TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  </Paper>
  );
}
*/
function convert_to_date(value) {
  return new Date(value).toLocaleString("en-US")
}

SimpleTable.propTypes = {
  classes: PropTypes.object.isRequired,
};

export default withStyles(styles)(SimpleTable);